use crate::action::{ActionCallback, StatefulActionMsg};
use crate::assets::{SPIN_DURATION, SPIN_STRATEGY};
#[cfg(feature = "benchmark")]
use crate::benchmark::Profiler;
use crate::config::{Config, LogCondition};
use crate::error;
use crate::error::Error;
use crate::error::Error::{FlowError, InternalError, LoggerError};
use crate::io::IO;
use crate::logger::{Logger, LoggerCallback};
use crate::scheduler::graph::{DependencyGraph, Edge, Node};
use crate::scheduler::info::Info;
use crate::scheduler::monitor::{Event, Monitor};
use crate::server::{Server, ServerCallback};
use crate::signal::{QReader, QWriter};
use chrono::{DateTime, Local};
use eframe::egui;
use eframe::egui::{CentralPanel, Color32, CursorIcon, RichText};
use itertools::Itertools;
use num_traits::Zero;
use petgraph::prelude::EdgeRef;
use petgraph::stable_graph::NodeIndex;
use petgraph::EdgeDirection;
use serde_json::Value;
use spin_sleep::SpinSleeper;
use std::collections::HashSet;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

pub mod flow;
pub mod graph;
pub mod info;
pub mod monitor;

#[derive(Debug, Clone)]
pub enum SyncCallback {
    UpdateGraph,
    KeyPress(HashSet<egui::Key>),
    Finish,
}

#[derive(Debug, Clone)]
pub enum AsyncCallback {
    Logger(DateTime<Local>, LoggerCallback),
    Finish,
}

#[derive(Debug)]
pub struct Scheduler {
    atomic: Arc<Mutex<(DependencyGraph, Vec<NodeIndex<usize>>, Option<usize>)>>,
    running: bool,
    info: Info,
    last_esc: Option<SystemTime>,
    config: Config,
    _io: IO,
    sync_qw: QWriter<SyncCallback>,
    async_qw: QWriter<AsyncCallback>,
    server_qw: QWriter<ServerCallback>,
    #[cfg(feature = "benchmark")]
    profiler: Profiler,
}

impl Scheduler {
    pub fn new(server: &Server) -> Result<Self, error::Error> {
        let task = server.task();
        let block = server.active_block();
        let actions = block.actions();
        let flow = block.flow();
        let resources = server.resources();

        let info = Info::new(server, task, block);

        let io = IO::new()?;
        let config = block.config(server.config());
        let (graph, nodes) = DependencyGraph::new(actions, flow, resources, &config, &io)?;
        let atomic = Arc::new(Mutex::new((graph, nodes, None)));

        let mut sync_qr = QReader::new();
        let mut sync_qw = sync_qr.writer();
        let mut async_qr = QReader::new();
        let mut async_qw = async_qr.writer();
        let mut server_qw = server.callback_channel();

        {
            let mut logger = Logger::new(&info, &config)?;
            let mut async_qw = async_qw.clone();

            thread::spawn(move || loop {
                while let signal = async_qr.pop() {
                    match signal {
                        AsyncCallback::Logger(time, callback) => {
                            logger.update(time, callback, &mut async_qw).unwrap();
                        }
                        AsyncCallback::Finish => {
                            break;
                        }
                    }
                }

                SpinSleeper::new(SPIN_DURATION)
                    .with_spin_strategy(SPIN_STRATEGY)
                    .sleep(Duration::from_millis(50));
            });
        }

        {
            let mut atomic = atomic.clone();
            let mut ready = flow.origin();
            let mut sync_qw = sync_qw.clone();
            let mut async_qw = async_qw.clone();
            let mut server_qw = server_qw.clone();

            thread::spawn(move || {
                let mut active = vec![];
                let mut timers = vec![];
                let mut key_monitors = HashSet::new();

                let timers2 = {
                    let (graph, nodes, foreground) = &mut (*atomic.lock().unwrap());
                    graph
                        .node_indices()
                        .filter_map(|v| graph.node(v).unwrap().start_timer.map(|t| (v.index(), t)))
                        .sorted_by_key(|(_, t)| *t)
                };

                async_qw.push(LoggerCallback::Append(
                    "mainevent".to_owned(),
                    ("start".to_owned(), Value::String("Success".to_owned())),
                ));

                let time = Instant::now();
                timers.extend(timers2.into_iter().map(|(v, t)| {
                    let mut sync_qw = sync_qw.clone();
                    thread::spawn(move || {
                        let offset = Instant::now() - time;
                        SpinSleeper::new(SPIN_DURATION)
                            .with_spin_strategy(SPIN_STRATEGY)
                            .sleep(t - offset);
                        sync_qw.push(SyncCallback::UpdateGraph);
                    });
                    (v, Edge::Starter, time.add(t))
                }));

                while let signal = sync_qr.pop() {
                    match signal {
                        SyncCallback::UpdateGraph => {
                            let (graph, nodes, foreground) = &mut (*atomic.lock().unwrap());

                            let mut done = vec![];
                            for &i in active.iter() {
                                if let Some(node) = graph.node_mut(nodes[i]) {
                                    if node.action.is_over()? {
                                        done.push(i);
                                    }
                                }
                            }

                            let mut ready2 = vec![];
                            for &i in ready.iter() {
                                if let Some(node) = graph.node(nodes[i]) {
                                    if node.action.is_over()? {
                                        done.push(i);
                                    } else {
                                        ready2.push(i);
                                    }
                                }
                            }
                            ready = ready2;

                            let time = Instant::now();
                            timers.retain(|(v, e, _)| match e {
                                Edge::Starter => graph.contains_node(nodes[*v]),
                                Edge::Stopper => active.contains(v),
                            });
                            for (v, e, _) in timers.iter().take_while(|(_, _, t)| time >= *t) {
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
                            timers.retain(|(_, _, t)| time < *t);

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
                                        graph.edges_directed(w, EdgeDirection::Incoming).for_each(
                                            |e2| match (e2.source(), e2.weight()) {
                                                (v2, Edge::Starter) if v == v2 => {
                                                    this_starter = true
                                                }
                                                (_, Edge::Starter) => other_starter = true,
                                                (v2, Edge::Stopper) if v == v2 => {
                                                    this_stopper = true
                                                }
                                                (_, Edge::Stopper) => other_stopper = true,
                                            },
                                        );
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
                                            async_qw.push(LoggerCallback::Extend(
                                                "flow".to_owned(),
                                                vec![
                                                    ("stop".to_owned(), Value::Number(i.into())),
                                                    (
                                                        "stop".to_owned(),
                                                        Value::String(name.clone()),
                                                    ),
                                                ],
                                            ));
                                        } else {
                                            async_qw.push(LoggerCallback::Append(
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

                            active.retain(|&i| graph.contains_node(nodes[i]));

                            let mut dropped_foreground = false;
                            if let Some(i) = *foreground {
                                if !active.contains(&i) {
                                    dropped_foreground = true;
                                    *foreground = None;
                                }
                            }
                            key_monitors.retain(|i| active.contains(i));

                            if !ready.is_empty() {
                                let time = Instant::now();
                                ready.sort();
                                for &i in ready.iter() {
                                    let node = graph.node_mut(nodes[i]).ok_or_else(|| {
                                        InternalError(format!(
                                            "Tried to start action {i} which has already been dropped"
                                        ))
                                    })?;
                                    node.start(&mut sync_qw, &mut async_qw)?;

                                    if matches!(
                                        node.log_when,
                                        LogCondition::Start | LogCondition::StartAndStop
                                    ) {
                                        if let Some(name) = node.name() {
                                            async_qw.push(LoggerCallback::Extend(
                                                "flow".to_owned(),
                                                vec![
                                                    ("start".to_owned(), Value::Number(i.into())),
                                                    (
                                                        "start".to_owned(),
                                                        Value::String(name.clone()),
                                                    ),
                                                ],
                                            ));
                                        } else {
                                            async_qw.push(LoggerCallback::Append(
                                                "flow".to_owned(),
                                                ("start".to_owned(), Value::Number(i.into())),
                                            ));
                                        }
                                    }

                                    if let Some(duration) = node.stop_timer {
                                        let target_time = time.add(duration);
                                        timers.push((i, Edge::Stopper, target_time));
                                        let mut sync_queue = sync_qw.clone();
                                        thread::spawn(move || {
                                            SpinSleeper::new(SPIN_DURATION)
                                                .with_spin_strategy(SPIN_STRATEGY)
                                                .sleep(target_time - Instant::now());
                                            sync_queue.push(SyncCallback::UpdateGraph);
                                        });
                                    }

                                    if node.action.is_visual() {
                                        if let Some(j) = *foreground {
                                            if dropped_foreground {
                                                *foreground = Some(i);
                                            } else {
                                                Err(FlowError(format!(
                                                    "Two foreground actions `{}` and `{}` collided (there is an error in the flow logic).",
                                                    foreground.unwrap(),
                                                    i
                                                )))?;
                                            }
                                        } else {
                                            *foreground = Some(i);
                                        }
                                    }

                                    match node.action.monitors() {
                                        Some(Monitor::Keys) => {
                                            key_monitors.insert(i);
                                        }
                                        None => {}
                                    }
                                }

                                timers.sort_by_key(|&(_, _, t)| t);
                                timers.retain(|(_, _, t)| time < *t);
                                active.extend_from_slice(&ready);
                                ready.clear();
                            }

                            if active.is_empty() && timers.is_empty() {
                                if graph.node_count() == 0 {
                                    async_qw.push(LoggerCallback::Append(
                                        "mainevent".to_owned(),
                                        ("finish".to_owned(), Value::String("Success".to_owned())),
                                    ));

                                    server_qw.push(ServerCallback::BlockFinished);
                                } else {
                                    let remaining: Vec<_> =
                                        nodes.iter().filter(|&&i| graph.contains_node(i)).collect();
                                    server_qw.push(
                                        ServerCallback::BlockCrashed(
                                            FlowError(format!(
                                                "Action flow has concluded, but the following actions were never reached: {remaining:?}"
                                            ))
                                        )
                                    );
                                }
                            }

                            #[cfg(debug_assertions)]
                            println!("Active -> {:?}", active);
                        }
                        SyncCallback::KeyPress(keys) => {
                            if !key_monitors.is_empty() {
                                let (graph, nodes, foreground) = &mut (*atomic.lock().unwrap());

                                for &i in key_monitors.iter() {
                                    if let Some(node) = graph.node_mut(nodes[i]) {
                                        node.action.update(
                                            ActionCallback::KeyPress(keys.clone()),
                                            &mut sync_qw,
                                            &mut async_qw,
                                        )?;
                                    }
                                }
                            }
                        }
                        SyncCallback::Finish => {
                            break;
                        }
                    }
                }

                Result::<(), error::Error>::Ok(())
            });
        }

        async_qw.push(LoggerCallback::Extend(
            "mainevent".to_owned(),
            vec![
                ("info".to_owned(), serde_json::to_value(&info).unwrap()),
                ("config".to_owned(), serde_json::to_value(&config).unwrap()),
            ],
        ));

        Ok(Self {
            atomic,
            running: false,
            info,
            last_esc: None,
            config,
            _io: io,
            sync_qw,
            async_qw,
            server_qw,
            #[cfg(feature = "benchmark")]
            profiler: Profiler::new(
                "Scheduler",
                vec!["keys", "proc", "show"],
                Duration::from_secs(60),
            ),
        })
    }

    #[inline(always)]
    pub fn info(&self) -> &Info {
        &self.info
    }

    #[inline(always)]
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn start(&mut self) -> Result<(), error::Error> {
        if self.running {
            panic!("Tried to start a scheduler when it was already running.");
        }

        self.sync_qw.push(SyncCallback::UpdateGraph);
        self.running = true;

        Ok(())
    }

    pub fn request_interrupt(&mut self) {
        self.async_qw.push(LoggerCallback::Append(
            "mainevent".to_owned(),
            (
                "interrupt".to_owned(),
                Value::String("User request".to_owned()),
            ),
        ));

        self.server_qw.push(ServerCallback::BlockInterrupted);
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Result<(), error::Error> {
        #[cfg(feature = "benchmark")]
        self.profiler.step();

        #[cfg(feature = "benchmark")]
        self.profiler.tic(0);
        if ui.input().key_pressed(egui::Key::Escape) {
            let time = SystemTime::now();
            if let Some(t) = self.last_esc.take() {
                if time.duration_since(t).unwrap() < Duration::from_millis(300) {
                    #[cfg(feature = "benchmark")]
                    self.profiler.toc(0);
                    return Ok(self.request_interrupt());
                }
            }
            self.last_esc = Some(time);
        }

        let mut keys_pressed = ui.input().keys_down.clone();
        keys_pressed.retain(|k| ui.input().key_pressed(*k));
        if !keys_pressed.is_empty() {
            self.sync_qw.push(SyncCallback::KeyPress(keys_pressed))
        }
        #[cfg(feature = "benchmark")]
        self.profiler.toc(0);

        #[cfg(feature = "benchmark")]
        self.profiler.tic(2);
        {
            let (graph, nodes, foreground) = &mut (*self.atomic.lock().unwrap());
            if let Some(i) = *foreground {
                if let Some(node) = graph.node_mut(nodes[i]) {
                    let result = node.action.show(ui, &mut self.sync_qw, &mut self.async_qw);

                    if let Err(e) = &result {
                        self.async_qw.push(LoggerCallback::Append(
                            "mainevent".to_owned(),
                            ("crash".to_owned(), Value::String(format!("{e:#?}"))),
                        ));
                    }

                    #[cfg(feature = "benchmark")]
                    self.profiler.toc(2);
                    return result;
                }
            }
        }
        #[cfg(feature = "benchmark")]
        self.profiler.toc(2);

        ui.output().cursor_icon = CursorIcon::None;

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), error::Error> {
        {
            let (graph, nodes, foreground) = &mut (*self.atomic.lock().unwrap());
            *foreground = None;
            graph.clear();
        }
        self.running = false;
        self.sync_qw.push(SyncCallback::Finish);
        self.async_qw.push(AsyncCallback::Finish);
        Ok(())
    }

    #[inline(always)]
    pub fn is_running(&self) -> bool {
        self.running
    }
}
