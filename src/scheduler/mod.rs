use crate::action::{ActionCallback, StatefulActionMsg};
use crate::assets::{SPIN_DURATION, SPIN_STRATEGY};
use crate::callback::{CallbackQueue, Destination};
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
use std::thread;
use std::time::{Duration, Instant, SystemTime};

pub mod flow;
pub mod graph;
pub mod info;
pub mod monitor;

#[derive(Debug, Clone)]
pub enum SyncCallback {
    // Setup,
    // Advance,
    // Start,
    // Stop,
    // Logger(LoggerCallback),
    // LoggerError(String),
    // Relay(StatefulActionMsg),
    UpdateGraph,
}

#[derive(Debug, Clone)]
pub enum AsyncCallback {
    // Setup,
    // Advance,
    // Start,
    Stop,
    Logger(LoggerCallback),
    LoggerError(String),
    // Relay(StatefulActionMsg),
    // KeyPress(KeyCode),
    // Refresh(u32),
    // UpdateGraph,
}

// impl SchedulerMsg {
//     #[inline(always)]
//     pub fn wrap(self) -> SyncCallback {
//         SyncCallback::Relay(self)
//     }
// }

#[derive(Debug)]
pub struct Scheduler {
    graph: DependencyGraph,
    ready: Vec<usize>,
    active: Vec<usize>,
    timers: Vec<(usize, Edge, Instant)>,
    nodes: Vec<NodeIndex<usize>>,
    foreground: Option<usize>,
    running: bool,
    info: Info,
    last_esc: Option<SystemTime>,
    key_monitors: HashSet<usize>,
    capture_key: bool,
    animation_id: u32,
    config: Config,
    success: bool,
    _io: IO,
    sync_queue: CallbackQueue<SyncCallback>,
    async_queue: CallbackQueue<AsyncCallback>,
    server_queue: CallbackQueue<ServerCallback>,
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

        let sync_queue = CallbackQueue::new();
        let mut async_queue = CallbackQueue::new();
        let server_queue = server.callback_channel();

        {
            let mut logger = Logger::new(&info, &config)?;
            let mut async_queue = async_queue.clone();
            thread::spawn(move || loop {
                while let Some((_dest, callback)) = async_queue.pop() {
                    match callback {
                        AsyncCallback::Logger(callback) => {
                            logger.update(callback, &mut async_queue).unwrap();
                        }
                        AsyncCallback::LoggerError(_) => {}
                        AsyncCallback::Stop => {
                            logger
                                .update(LoggerCallback::Finish, &mut async_queue)
                                .unwrap();
                            return;
                        }
                    }
                }
                thread::sleep(Duration::from_millis(100));
            });
        }

        async_queue.push(
            Destination::default(),
            LoggerCallback::Extend(
                "mainevent".to_owned(),
                vec![
                    ("info".to_owned(), serde_json::to_value(&info).unwrap()),
                    ("config".to_owned(), serde_json::to_value(&config).unwrap()),
                ],
            ),
        );

        Ok(Self {
            graph,
            ready: flow.origin(),
            active: vec![],
            timers: vec![],
            nodes,
            foreground: None,
            running: false,
            info,
            last_esc: None,
            key_monitors: HashSet::new(),
            capture_key: false,
            animation_id: 0,
            config,
            success: false,
            _io: io,
            sync_queue,
            async_queue,
            server_queue,
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

    fn process(&mut self, callback: SyncCallback) -> Result<(), error::Error> {
        match callback {
            SyncCallback::UpdateGraph => {
                self.advance()
            }
        }
    }

    fn start_queue(&mut self) -> Result<(), error::Error> {
        let mut dropped_foreground = false;
        if let Some(i) = self.foreground {
            if !self.active.contains(&i) {
                dropped_foreground = true;
                self.foreground = None;
            }
        }
        self.key_monitors.retain(|i| self.active.contains(i));
        if self.key_monitors.is_empty() {
            self.capture_key = false;
        }

        if !self.ready.is_empty() {
            let time = Instant::now();
            self.ready.sort();
            for &i in self.ready.iter() {
                let node = self.graph.node_mut(self.nodes[i]).ok_or_else(|| {
                    InternalError(format!(
                        "Tried to start action {i} which has already been dropped"
                    ))
                })?;
                node.start(&mut self.sync_queue, &mut self.async_queue)?;

                if matches!(
                    node.log_when,
                    LogCondition::Start | LogCondition::StartAndStop
                ) {
                    if let Some(name) = node.name() {
                        self.async_queue.push(
                            Destination::default(),
                            LoggerCallback::Extend(
                                "flow".to_owned(),
                                vec![
                                    ("start".to_owned(), Value::Number(i.into())),
                                    ("start".to_owned(), Value::String(name.clone())),
                                ],
                            ),
                        );
                    } else {
                        self.async_queue.push(
                            Destination::default(),
                            LoggerCallback::Append(
                                "flow".to_owned(),
                                ("start".to_owned(), Value::Number(i.into())),
                            ),
                        );
                    }
                }

                if let Some(duration) = node.stop_timer {
                    let target_time = time.add(duration);
                    self.timers.push((i, Edge::Stopper, target_time));
                    let mut sync_queue = self.sync_queue.clone();
                    thread::spawn(move || {
                        SpinSleeper::new(SPIN_DURATION)
                            .with_spin_strategy(SPIN_STRATEGY)
                            .sleep(target_time - Instant::now());
                        sync_queue.push(Destination::default(), SyncCallback::UpdateGraph);
                    });
                }

                if node.action.is_visual() {
                    if self.foreground.is_none() || dropped_foreground {
                        self.foreground = Some(i);
                    } else {
                        Err(FlowError(format!(
                            "Two foreground actions `{}` and `{}` collided (there is an error in the flow logic).",
                            self.foreground.unwrap(),
                            i
                        )))?;
                    }
                }

                match node.action.monitors() {
                    Some(Monitor::Keys) => {
                        self.key_monitors.insert(i);
                        self.capture_key = true;
                    }
                    None => {}
                }
            }

            self.timers.sort_by_key(|&(_, _, t)| t);
            self.timers.retain(|(_, _, t)| time < *t);
            self.active.extend_from_slice(&self.ready);
            self.ready.clear();
        }

        if self.active.is_empty() && self.timers.is_empty() {
            if self.graph.node_count() == 0 {
                self.success = true;
                self.request_finish();
                Ok(())
            } else {
                let remaining: Vec<_> = self
                    .nodes
                    .iter()
                    .filter(|&&i| self.graph.contains_node(i))
                    .collect();
                Err(FlowError(format!(
                    "Action flow has concluded, but the following actions were never reached: {remaining:?}"
                )))
            }
        } else {
            Ok(())
        }
    }

    pub fn start(&mut self) -> Result<(), error::Error> {
        if self.running || !self.active.is_empty() {
            panic!("Tried to start a scheduler when it was already running.");
        }

        let timers = self
            .graph
            .node_indices()
            .filter_map(|v| {
                self.graph
                    .node(v)
                    .unwrap()
                    .start_timer
                    .map(|t| (v.index(), t))
            })
            .sorted_by_key(|(_, t)| *t);

        self.async_queue.push(
            Destination::default(),
            LoggerCallback::Append(
                "mainevent".to_owned(),
                ("start".to_owned(), Value::String("Success".to_owned())),
            ),
        );

        let time = Instant::now();
        self.running = true;
        self.timers.extend(timers.into_iter().map(|(v, t)| {
            let mut sync_queue = self.sync_queue.clone();
            thread::spawn(move || {
                let offset = Instant::now() - time;
                SpinSleeper::new(SPIN_DURATION)
                    .with_spin_strategy(SPIN_STRATEGY)
                    .sleep(t - offset);
                sync_queue.push(Destination::default(), SyncCallback::UpdateGraph);
            });
            (v, Edge::Starter, time.add(t))
        }));

        self.sync_queue
            .push(Destination::default(), SyncCallback::UpdateGraph);
        Ok(())
    }

    pub fn advance(&mut self) -> Result<(), error::Error> {
        let mut done = vec![];
        for &i in self.active.iter() {
            if let Some(node) = self.graph.node_mut(self.nodes[i]) {
                if node.action.is_over()? {
                    done.push(i);
                }
            }
        }

        let mut ready = vec![];
        for &i in self.ready.iter() {
            if let Some(node) = self.graph.node(self.nodes[i]) {
                if node.action.is_over()? {
                    done.push(i);
                } else {
                    ready.push(i);
                }
            }
        }

        let time = Instant::now();
        self.timers.retain(|(v, e, _)| match e {
            Edge::Starter => self.graph.contains_node(self.nodes[*v]),
            Edge::Stopper => self.active.contains(v),
        });
        for (v, e, _) in self.timers.iter().take_while(|(_, _, t)| time >= *t) {
            match e {
                Edge::Starter => {
                    if let Some(Node { action, .. }) = self.graph.node(self.nodes[*v]) {
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
                let v = self.nodes[v];
                self.graph.edges(v).for_each(|e| {
                    let w = e.target();
                    let mut this_starter = false;
                    let mut other_starter = false;
                    let mut this_stopper = false;
                    let mut other_stopper = false;
                    self.graph
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

                let node = self.graph.remove_node(v).ok_or_else(|| {
                    InternalError(format!(
                        "Tried to drop action `{i}` which has already been dropped"
                    ))
                })?;

                if matches!(
                    node.log_when,
                    LogCondition::Stop | LogCondition::StartAndStop
                ) {
                    if let Some(name) = node.name() {
                        self.async_queue.push(
                            Destination::default(),
                            LoggerCallback::Extend(
                                "flow".to_owned(),
                                vec![
                                    ("stop".to_owned(), Value::Number(i.into())),
                                    ("stop".to_owned(), Value::String(name.clone())),
                                ],
                            ),
                        );
                    } else {
                        self.async_queue.push(
                            Destination::default(),
                            LoggerCallback::Append(
                                "flow".to_owned(),
                                ("stop".to_owned(), Value::Number(i.into())),
                            ),
                        );
                    }
                }

                #[cfg(debug_assertions)]
                println!("[=> Action complete {} @ {:?}]", v.index(), time);
            }
            done = over;
        }

        self.active
            .retain(|&i| self.graph.contains_node(self.nodes[i]));
        self.ready = ready;
        self.start_queue()?;
        #[cfg(debug_assertions)]
        println!("Active -> {:?}", self.active);
        Ok(())
    }

    pub fn request_finish(&mut self) {
        self.async_queue.push(
            Destination::default(),
            LoggerCallback::Append(
                "mainevent".to_owned(),
                ("finish".to_owned(), Value::String("Success".to_owned())),
            ),
        );

        self.server_queue
            .push(Destination::default(), ServerCallback::BlockFinished);
    }

    pub fn request_interrupt(&mut self) {
        self.async_queue.push(
            Destination::default(),
            LoggerCallback::Append(
                "mainevent".to_owned(),
                (
                    "interrupt".to_owned(),
                    Value::String("User request".to_owned()),
                ),
            ),
        );

        self.server_queue
            .push(Destination::default(), ServerCallback::BlockInterrupted);
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Result<(), error::Error> {
        if ctx.input().key_pressed(egui::Key::Escape) {
            let time = SystemTime::now();
            if let Some(t) = self.last_esc.take() {
                if time.duration_since(t).unwrap() < Duration::from_millis(300) {
                    return Ok(self.request_interrupt());
                }
            }
            self.last_esc = Some(time);
        }

        let mut keys_pressed = ctx.input().keys_down.clone();
        keys_pressed.retain(|k| ctx.input().key_pressed(*k));
        for &i in self.key_monitors.iter() {
            if let Some(node) = self.graph.node_mut(self.nodes[i]) {
                node.action.update(
                    ActionCallback::KeyPress(keys_pressed.clone()),
                    &mut self.sync_queue,
                    &mut self.async_queue,
                )?;
            }
        }

        let mut updated_graph = false;
        while let Some((_dest, callback)) = self.sync_queue.pop() {
            match callback {
                SyncCallback::UpdateGraph => {
                    if !updated_graph {
                        self.process(callback)?;
                        updated_graph = true;
                    }
                }
            }
        }

        if let Some(i) = self.foreground {
            if let Some(node) = self.graph.node_mut(self.nodes[i]) {
                let result = node
                    .action
                    .show(ctx, &mut self.sync_queue, &mut self.async_queue);

                if let Err(e) = &result {
                    self.async_queue.push(
                        Destination::default(),
                        LoggerCallback::Append(
                            "mainevent".to_owned(),
                            ("crash".to_owned(), Value::String(format!("{e:#?}"))),
                        ),
                    );
                }

                return result;
            }
        }

        CentralPanel::default().show(ctx, |ui| {
            ui.output().cursor_icon = CursorIcon::None;
        });
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), error::Error> {
        self.running = false;
        self.foreground = None;
        self.ready.clear();
        self.active.clear();
        self.timers.clear();
        self.nodes.clear();
        self.key_monitors.clear();
        self.capture_key = false;
        self.graph.clear();

        self.async_queue
            .push(Destination::default(), AsyncCallback::Stop);

        Ok(())
    }

    #[inline(always)]
    pub fn is_running(&self) -> bool {
        self.running
    }

    #[inline(always)]
    pub fn captures_key(&self) -> bool {
        self.capture_key
    }

    #[inline(always)]
    pub fn animation_id(&self) -> u32 {
        self.animation_id
    }
}
