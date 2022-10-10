use crate::action::StatefulActionMsg;
use crate::assets::{SPIN_DURATION, SPIN_STRATEGY};
use crate::config::{Config, LogCondition};
use crate::error;
use crate::error::Error::{FlowError, InternalError, LoggerError};
use crate::io::IO;
use crate::logger::{Logger, LoggerCallback};
use crate::scheduler::graph::{DependencyGraph, Edge, Node};
use crate::scheduler::info::Info;
use crate::scheduler::monitor::{Event, Monitor};
use crate::server::{Server, SyncCallback};
use iced::keyboard::KeyCode;
use iced::pure::widget::Column;
use iced::pure::Element;
use iced::Command;
use itertools::Itertools;
use num_traits::Zero;
use petgraph::prelude::EdgeRef;
use petgraph::stable_graph::NodeIndex;
use petgraph::EdgeDirection;
use serde_json::Value;
use spin_sleep::SpinSleeper;
use std::collections::HashSet;
use std::ops::Add;
use std::time::{Duration, Instant, SystemTime};

pub mod flow;
pub mod graph;
pub mod info;
pub mod monitor;

#[derive(Debug, Clone)]
pub enum SchedulerMsg {
    Setup,
    Advance,
    Start,
    Stop,
    Logger(LoggerCallback),
    LoggerError(String),
    Relay(StatefulActionMsg),
    KeyPress(KeyCode),
    Refresh(u32),
}

impl SchedulerMsg {
    #[inline(always)]
    pub fn wrap(self) -> SyncCallback {
        SyncCallback::Relay(self)
    }
}

#[derive(Debug)]
pub struct Scheduler {
    graph: DependencyGraph,
    ready: Vec<usize>,
    active: Vec<usize>,
    timers: Vec<(usize, Edge, Instant)>,
    nodes: Vec<NodeIndex<usize>>,
    foreground: Option<usize>,
    logger: Option<Logger>,
    running: bool,
    needs_refresh: bool,
    info: Info,
    last_esc: Option<SystemTime>,
    key_monitors: HashSet<usize>,
    fps_monitors: HashSet<usize>,
    capture_key: bool,
    capture_fps: Option<f64>,
    animation_id: u32,
    config: Config,
    success: bool,
    _io: IO,
}

impl Scheduler {
    pub fn new(server: &Server) -> Result<(Self, Command<SyncCallback>), error::Error> {
        let task = server.task();
        let block = server.active_block();
        let actions = block.actions();
        let flow = block.flow();
        let resources = server.resources();

        let info = Info::new(server, task, block);

        let io = IO::new()?;
        let config = block.config(server.config());
        let (graph, nodes) = DependencyGraph::new(actions, flow, resources, &config, &io)?;

        let mut logger = Logger::new(&info, &config)?;
        let cmd = logger.update(LoggerCallback::Extend(
            "mainevent".to_owned(),
            vec![
                ("info".to_owned(), serde_json::to_value(&info).unwrap()),
                ("config".to_owned(), serde_json::to_value(&config).unwrap()),
            ],
        ))?;

        Ok((
            Self {
                graph,
                ready: flow.origin(),
                active: vec![],
                timers: vec![],
                nodes,
                foreground: None,
                logger: Some(logger),
                running: false,
                needs_refresh: false,
                info,
                last_esc: None,
                key_monitors: HashSet::new(),
                fps_monitors: HashSet::new(),
                capture_key: false,
                capture_fps: None,
                animation_id: 0,
                config,
                success: false,
                _io: io,
            },
            cmd,
        ))
    }

    #[inline(always)]
    pub fn info(&self) -> &Info {
        &self.info
    }

    #[inline(always)]
    pub fn config(&self) -> &Config {
        &self.config
    }

    fn start_queue(&mut self) -> Result<Command<SyncCallback>, error::Error> {
        let mut cmd = vec![];
        let mut dropped_foreground = false;
        if let Some(i) = self.foreground {
            if !self.active.contains(&i) {
                dropped_foreground = true;
                self.foreground = None;
                self.needs_refresh = true;
            }
        }
        self.key_monitors.retain(|i| self.active.contains(i));
        if self.key_monitors.is_empty() {
            self.capture_key = false;
        }
        self.fps_monitors.retain(|i| self.active.contains(i));
        if self.fps_monitors.is_empty() {
            self.capture_fps = None;
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
                cmd.push(node.start()?);

                if matches!(
                    node.log_when,
                    LogCondition::Start | LogCondition::StartAndStop
                ) {
                    let logger = self.logger.as_mut().expect("Failed to fetch logger");
                    if let Some(name) = node.name() {
                        cmd.push(logger.update(LoggerCallback::Extend(
                            "flow".to_owned(),
                            vec![
                                ("start".to_owned(), Value::Number(i.into())),
                                ("start".to_owned(), Value::String(name.clone())),
                            ],
                        ))?);
                    } else {
                        cmd.push(logger.update(LoggerCallback::Append(
                            "flow".to_owned(),
                            ("start".to_owned(), Value::Number(i.into())),
                        ))?);
                    }
                }

                if let Some(duration) = node.stop_timer {
                    let target_time = time.add(duration);
                    self.timers.push((i, Edge::Stopper, target_time));
                    cmd.push(Command::perform(
                        async move {
                            SpinSleeper::new(SPIN_DURATION)
                                .with_spin_strategy(SPIN_STRATEGY)
                                .sleep(target_time - Instant::now());
                            SchedulerMsg::Advance
                        },
                        SchedulerMsg::wrap,
                    ));
                }

                if node.action.is_visual() {
                    if self.foreground.is_none() || dropped_foreground {
                        self.foreground = Some(i);
                        self.needs_refresh = true;
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
                    Some(Monitor::Frames(fps)) => {
                        if self.fps_monitors.is_empty() {
                            self.animation_id += 1;
                        }
                        self.fps_monitors.insert(i);
                        match self.capture_fps {
                            None => {
                                let base_fps = self.config.fps_lock();
                                self.capture_fps = Some(
                                    if base_fps.is_zero() {
                                        (2.0 * fps).min(40.0)
                                    } else {
                                        base_fps
                                    }
                                );
                            },
                            Some(f) if self.config.fps_lock().is_zero() || (f - fps).abs() < 1e-6 => {}
                            _ => Err(FlowError("Cannot play two animations with different frame rates simultaneously".to_owned()))?
                        }
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
                Ok(self.request_finish())
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
            Ok(Command::batch(cmd))
        }
    }

    pub fn start(&mut self) -> Result<Command<SyncCallback>, error::Error> {
        let mut cmd = vec![];
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

        let time = Instant::now();
        self.running = true;
        self.timers.extend(timers.into_iter().map(|(v, t)| {
            cmd.push(Command::perform(
                async move {
                    let offset = Instant::now() - time;
                    SpinSleeper::new(SPIN_DURATION)
                        .with_spin_strategy(SPIN_STRATEGY)
                        .sleep(t - offset);
                    SchedulerMsg::Advance
                },
                SchedulerMsg::wrap,
            ));
            (v, Edge::Starter, time.add(t))
        }));

        self.needs_refresh = true;
        cmd.push(Command::perform(async {}, |()| {
            SchedulerMsg::Advance.wrap()
        }));
        Ok(Command::batch(cmd))
    }

    pub fn advance(&mut self) -> Result<Command<SyncCallback>, error::Error> {
        let mut cmd = vec![];
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
                    let logger = self.logger.as_mut().expect("Failed to fetch logger");

                    if let Some(name) = node.name() {
                        cmd.push(logger.update(LoggerCallback::Extend(
                            "flow".to_owned(),
                            vec![
                                ("stop".to_owned(), Value::Number(i.into())),
                                ("stop".to_owned(), Value::String(name.clone())),
                            ],
                        ))?);
                    } else {
                        cmd.push(logger.update(LoggerCallback::Append(
                            "flow".to_owned(),
                            ("stop".to_owned(), Value::Number(i.into())),
                        ))?);
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
        if cmd.is_empty() {
            self.start_queue()
        } else {
            Ok(Command::batch([self.start_queue()?, Command::batch(cmd)]))
        }
    }

    pub fn request_finish(&mut self) -> Command<SyncCallback> {
        if let Some(logger) = &mut self.logger {
            Command::batch([
                logger
                    .update(LoggerCallback::Append(
                        "mainevent".to_owned(),
                        ("finish".to_owned(), Value::String("Success".to_owned())),
                    ))
                    .unwrap(),
                Command::perform(async {}, |()| SyncCallback::BlockFinished),
            ])
        } else {
            Command::perform(async {}, |()| SyncCallback::BlockFinished)
        }
    }

    pub fn request_interrupt(&mut self) -> Command<SyncCallback> {
        if let Some(logger) = &mut self.logger {
            Command::batch([
                logger
                    .update(LoggerCallback::Append(
                        "mainevent".to_owned(),
                        (
                            "interrupt".to_owned(),
                            Value::String("User request".to_owned()),
                        ),
                    ))
                    .unwrap(),
                Command::perform(async {}, |()| SyncCallback::BlockInterrupted),
            ])
        } else {
            Command::perform(async {}, |()| SyncCallback::BlockInterrupted)
        }
    }

    pub fn update(&mut self, msg: SchedulerMsg) -> Result<Command<SyncCallback>, error::Error> {
        self.needs_refresh = false;
        match (self.running, msg) {
            (false, SchedulerMsg::Start) => {
                if let Some(logger) = &mut self.logger {
                    Ok(Command::batch([
                        logger.update(LoggerCallback::Append(
                            "mainevent".to_owned(),
                            ("start".to_owned(), Value::String("Success".to_owned())),
                        ))?,
                        self.start()?,
                    ]))
                } else {
                    self.start()
                }
            }
            (true, SchedulerMsg::Advance) => self.advance(),
            (true, SchedulerMsg::Relay(msg)) => {
                if let Some(i) = self.foreground {
                    if let Some(node) = self.graph.node_mut(self.nodes[i]) {
                        return node.action.update(msg);
                    }
                }
                Ok(Command::none())
            }
            (true, SchedulerMsg::Logger(msg)) => {
                if let Some(logger) = self.logger.as_mut() {
                    logger.update(msg)
                } else {
                    #[cfg(debug_assertions)]
                    println!("WW: Tried to send message to non-existent logger");
                    Ok(Command::none())
                }
            }
            (true, SchedulerMsg::LoggerError(msg)) => Err(LoggerError(msg)),
            (true, SchedulerMsg::Stop) => self.stop(),
            (true, SchedulerMsg::KeyPress(key)) => {
                if key == KeyCode::Escape {
                    let time = SystemTime::now();
                    if let Some(t) = self.last_esc.take() {
                        if time.duration_since(t).unwrap() < Duration::from_millis(300) {
                            return Ok(self.request_interrupt());
                        }
                    }
                    self.last_esc = Some(time);
                }

                let mut cmd = vec![];
                for &i in self.key_monitors.iter() {
                    if let Some(node) = self.graph.node_mut(self.nodes[i]) {
                        cmd.push(
                            node.action
                                .update(StatefulActionMsg::UpdateEvent(Event::Key(key)))?,
                        );
                    }
                }
                Ok(Command::batch(cmd))
            }
            _ => Ok(Command::none()),
        }
    }

    pub fn view(&self, scale_factor: f32) -> Result<Element<'_, SyncCallback>, error::Error> {
        if let Some(i) = self.foreground {
            if let Some(node) = self.graph.node(self.nodes[i]) {
                return node.action.view(scale_factor);
            }
        }
        Ok(Column::new().into())
    }

    pub fn stop(&mut self) -> Result<Command<SyncCallback>, error::Error> {
        self.running = false;
        self.foreground = None;
        self.ready.clear();
        self.active.clear();
        self.timers.clear();
        self.nodes.clear();
        self.key_monitors.clear();
        self.fps_monitors.clear();
        self.capture_key = false;
        self.capture_fps = None;
        self.graph.clear();
        self.needs_refresh = true;

        if let Some(logger) = self.logger.as_mut() {
            Ok(Command::batch([
                logger.update(LoggerCallback::Finish)?,
                Command::perform(async {}, move |()| SyncCallback::CleanupComplete(Ok(()))),
            ]))
        } else {
            Ok(Command::perform(async {}, move |()| {
                SyncCallback::CleanupComplete(Ok(()))
            }))
        }
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
    pub fn captures_fps(&self) -> Option<f64> {
        self.capture_fps
    }

    #[inline(always)]
    pub fn animation_id(&self) -> u32 {
        self.animation_id
    }

    #[inline(always)]
    pub fn needs_refresh(&self) -> bool {
        self.needs_refresh
    }
}
