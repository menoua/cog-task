use crate::action::ActionSignal;
use crate::config::{Config, LogCondition};
use crate::error;
use crate::error::Error::{FlowError, InternalError};
use crate::logger::{Logger, LoggerSignal};
use crate::scheduler::flow::Flow;
use crate::scheduler::graph::{DependencyGraph, Edge, Node};
use crate::scheduler::info::Info;
use crate::server::ServerSignal;
use crate::signal::{QReader, QWriter};
use crate::util::spin_sleeper;
use chrono::{DateTime, Local};
use eframe::egui;
use itertools::Itertools;
use petgraph::prelude::{EdgeRef, NodeIndex};
use petgraph::EdgeDirection;
use serde_json::Value;
use std::collections::HashSet;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub type Atomic = Arc<Mutex<(DependencyGraph, Vec<NodeIndex<usize>>, Option<usize>)>>;
pub type Timer = (usize, Edge, Instant);

#[derive(Debug, Clone)]
pub enum AsyncSignal {
    Logger(DateTime<Local>, LoggerSignal),
    Finish,
}

pub struct AsyncProcessor {
    logger: Logger,
    async_reader: QReader<AsyncSignal>,
    async_writer: QWriter<AsyncSignal>,
    server_writer: QWriter<ServerSignal>,
}

#[derive(Debug, Clone)]
pub enum SyncSignal {
    UpdateGraph,
    KeyPress(HashSet<egui::Key>),
    Finish,
}

pub struct SyncProcessor {
    ctx: egui::Context,
    atomic: Atomic,
    ready: Vec<usize>,
    active: Vec<usize>,
    timers: Vec<Timer>,
    key_monitors: HashSet<usize>,
    sync_reader: QReader<SyncSignal>,
    sync_writer: QWriter<SyncSignal>,
    async_writer: QWriter<AsyncSignal>,
    server_writer: QWriter<ServerSignal>,
}

impl AsyncProcessor {
    pub fn spawn(
        info: &Info,
        config: &Config,
        server_writer: &QWriter<ServerSignal>,
    ) -> Result<QWriter<AsyncSignal>, error::Error> {
        let async_reader = QReader::new();
        let async_writer = async_reader.writer();
        let mut proc = Self {
            logger: Logger::new(info, config)?,
            async_reader,
            async_writer,
            server_writer: server_writer.clone(),
        };

        let async_writer = proc.async_writer.clone();

        thread::spawn(move || {
            while let Some(signal) = proc.async_reader.pop() {
                match signal {
                    AsyncSignal::Logger(time, signal) => {
                        proc.logger
                            .update(time, signal, &proc.async_writer)
                            .unwrap();
                    }
                    AsyncSignal::Finish => break,
                }
            }

            let result = proc.logger.finish();
            proc.server_writer.push(ServerSignal::AsyncComplete(result));
        });

        Ok(async_writer)
    }
}

impl SyncProcessor {
    pub fn spawn(
        ctx: &egui::Context,
        graph: DependencyGraph,
        nodes: Vec<NodeIndex<usize>>,
        flow: &Flow,
        async_writer: &QWriter<AsyncSignal>,
        server_writer: &QWriter<ServerSignal>,
    ) -> Result<(QWriter<SyncSignal>, Atomic), error::Error> {
        let sync_reader = QReader::new();
        let sync_writer = sync_reader.writer();
        let mut proc = Self {
            ctx: ctx.clone(),
            atomic: Arc::new(Mutex::new((graph, nodes, None))),
            ready: flow.origin(),
            active: vec![],
            timers: vec![],
            key_monitors: HashSet::new(),
            sync_reader,
            sync_writer,
            async_writer: async_writer.clone(),
            server_writer: server_writer.clone(),
        };

        let sync_writer = proc.sync_writer.clone();
        let atomic = proc.atomic.clone();

        thread::spawn(move || {
            proc.init();

            while let Some(signal) = proc.sync_reader.pop() {
                match signal {
                    SyncSignal::UpdateGraph => proc.update_graph(),
                    SyncSignal::KeyPress(keys) => proc.update_keypress(keys),
                    SyncSignal::Finish => break,
                }
                .unwrap_or_else(|e| {
                    proc.server_writer.push(ServerSignal::BlockCrashed(e));
                });
            }

            proc.atomic.lock().unwrap().0.clear();
            proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
        });

        Ok((sync_writer, atomic))
    }

    fn init(&mut self) {
        let (graph, _, _) = &mut (*self.atomic.lock().unwrap());

        let timers = graph
            .node_indices()
            .filter_map(|v| graph.node(v).unwrap().start_timer.map(|t| (v.index(), t)))
            .sorted_by_key(|(_, t)| *t);

        self.async_writer.push(LoggerSignal::Append(
            "mainevent".to_owned(),
            ("start".to_owned(), Value::String("Success".to_owned())),
        ));

        let time = Instant::now();
        self.timers.extend(timers.into_iter().map(|(v, t)| {
            let mut sync_writer = self.sync_writer.clone();
            thread::spawn(move || {
                let offset = Instant::now() - time;
                spin_sleeper().sleep(t - offset);
                sync_writer.push(SyncSignal::UpdateGraph);
            });
            (v, Edge::Starter, time.add(t))
        }));
    }

    fn update_graph(&mut self) -> Result<(), error::Error> {
        {
            let (graph, nodes, _) = &mut (*self.atomic.lock().unwrap());

            let mut done = vec![];
            for &i in self.active.iter() {
                if let Some(node) = graph.node_mut(nodes[i]) {
                    if node.action.is_over()? {
                        done.push(i);
                    }
                }
            }

            let mut ready = vec![];
            for &i in self.ready.iter() {
                if let Some(node) = graph.node(nodes[i]) {
                    if node.action.is_over()? {
                        done.push(i);
                    } else {
                        ready.push(i);
                    }
                }
            }

            let time = Instant::now();
            self.timers.retain(|(v, e, _)| match e {
                Edge::Starter => graph.contains_node(nodes[*v]),
                Edge::Stopper => self.active.contains(v),
            });
            for (v, e, _) in self.timers.iter().take_while(|(_, _, t)| time >= *t) {
                match e {
                    Edge::Starter => {
                        if let Some(Node { action, .. }) = graph.node(nodes[*v]) {
                            if action.is_over()? {
                                done.push(*v);
                            } else {
                                ready.push(*v);
                            }
                        }
                    }
                    Edge::Stopper => done.push(*v),
                }
            }
            self.timers.retain(|(_, _, t)| time < *t);

            while !done.is_empty() {
                let mut over = vec![];
                for &v in done.iter() {
                    let i = v;
                    let v = nodes[v];
                    graph.edges(v).for_each(|e| {
                        let w = e.target();
                        let mut this_starter = false;
                        let mut other_starter = false;
                        let mut this_stopper = false;
                        let mut other_stopper = false;
                        graph
                            .edges_directed(w, EdgeDirection::Incoming)
                            .for_each(|e2| match (e2.source(), e2.weight()) {
                                (v2, Edge::Starter) if v == v2 => this_starter = true,
                                (_, Edge::Starter) => other_starter = true,
                                (v2, Edge::Stopper) if v == v2 => this_stopper = true,
                                (_, Edge::Stopper) => other_stopper = true,
                            });
                        if this_starter && !other_starter {
                            ready.push(w.index());
                        } else if this_stopper && !other_stopper {
                            over.push(w.index());
                        }
                    });

                    let node = graph.remove_node(v).ok_or_else(|| {
                        InternalError(format!(
                            "Tried to drop action `{i}` which has already been dropped"
                        ))
                    })?;

                    if matches!(
                        node.log_when,
                        LogCondition::Stop | LogCondition::StartAndStop
                    ) {
                        if let Some(name) = node.name() {
                            self.async_writer.push(LoggerSignal::Extend(
                                "flow".to_owned(),
                                vec![
                                    ("stop".to_owned(), Value::Number(i.into())),
                                    ("stop".to_owned(), Value::String(name.clone())),
                                ],
                            ));
                        } else {
                            self.async_writer.push(LoggerSignal::Append(
                                "flow".to_owned(),
                                ("stop".to_owned(), Value::Number(i.into())),
                            ));
                        }
                    }

                    #[cfg(debug_assertions)]
                    println!("[=> Action complete {} @ {:?}]", v.index(), time);
                }
                done = over;
            }

            self.active.retain(|&i| graph.contains_node(nodes[i]));
            self.ready = ready;
        }

        self.start_ready()
    }

    fn start_ready(&mut self) -> Result<(), error::Error> {
        let (graph, nodes, foreground) = &mut (*self.atomic.lock().unwrap());
        let mut dropped_foreground = false;
        let mut new_foreground = false;

        if let Some(i) = *foreground {
            if !self.active.contains(&i) {
                dropped_foreground = true;
                *foreground = None;
            }
        }
        self.key_monitors.retain(|i| self.active.contains(i));

        if !self.ready.is_empty() {
            let time = Instant::now();
            self.ready.sort();
            for &i in self.ready.iter() {
                let node = graph.node_mut(nodes[i]).ok_or_else(|| {
                    InternalError(format!(
                        "Tried to start action {i} which has already been dropped"
                    ))
                })?;

                node.start(&mut self.sync_writer, &mut self.async_writer)?;

                if matches!(
                    node.log_when,
                    LogCondition::Start | LogCondition::StartAndStop
                ) {
                    if let Some(name) = node.name() {
                        self.async_writer.push(LoggerSignal::Extend(
                            "flow".to_owned(),
                            vec![
                                ("start".to_owned(), Value::Number(i.into())),
                                ("start".to_owned(), Value::String(name.clone())),
                            ],
                        ));
                    } else {
                        self.async_writer.push(LoggerSignal::Append(
                            "flow".to_owned(),
                            ("start".to_owned(), Value::Number(i.into())),
                        ));
                    }
                }

                if let Some(duration) = node.stop_timer {
                    let target_time = time.add(duration);
                    self.timers.push((i, Edge::Stopper, target_time));
                    let mut sync_writer = self.sync_writer.clone();
                    thread::spawn(move || {
                        spin_sleeper().sleep(target_time - Instant::now());
                        sync_writer.push(SyncSignal::UpdateGraph);
                    });
                }

                if node.action.props().visual() {
                    if let Some(j) = *foreground {
                        if dropped_foreground {
                            *foreground = Some(i);
                        } else {
                            Err(FlowError(format!(
                                "Two foreground actions `{j}` and `{i}` collided (there is an error in the flow logic)."
                            )))?;
                        }
                    } else {
                        *foreground = Some(i);
                    }

                    new_foreground = true;
                }

                if node.action.props().captures_keys() {
                    self.key_monitors.insert(i);
                }
            }

            self.timers.sort_by_key(|&(_, _, t)| t);
            self.timers.retain(|(_, _, t)| time < *t);
            self.active.extend_from_slice(&self.ready);
            self.ready.clear();
        }

        if self.active.is_empty() && self.timers.is_empty() {
            if graph.node_count() == 0 {
                self.async_writer.push(LoggerSignal::Append(
                    "mainevent".to_owned(),
                    ("finish".to_owned(), Value::String("Success".to_owned())),
                ));

                self.server_writer.push(ServerSignal::BlockFinished);
            } else {
                let remaining: Vec<_> = nodes.iter().filter(|&&i| graph.contains_node(i)).collect();
                self.server_writer.push(
                    ServerSignal::BlockCrashed(
                        FlowError(format!(
                            "Action flow has concluded, but the following actions were never reached: {remaining:?}"
                        ))
                    )
                );
            }
        }

        if dropped_foreground || new_foreground {
            self.ctx.request_repaint();
        }

        #[cfg(debug_assertions)]
        println!("Active -> {:?}", self.active);
        Ok(())
    }

    fn update_keypress(&mut self, keys: HashSet<egui::Key>) -> Result<(), error::Error> {
        if !self.key_monitors.is_empty() {
            let (graph, nodes, _) = &mut (*self.atomic.lock().unwrap());

            for &i in self.key_monitors.iter() {
                if let Some(node) = graph.node_mut(nodes[i]) {
                    node.action.update(
                        ActionSignal::KeyPress(keys.clone()),
                        &mut self.sync_writer,
                        &mut self.async_writer,
                    )?;
                }
            }
        }

        Ok(())
    }
}
