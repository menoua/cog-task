#[macro_use]
mod macros;
pub mod core;
pub mod de;
pub mod extra;
pub mod include;
pub mod props;
pub mod resource;

pub use include::*;
pub use props::*;

use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::Key;
use crate::server::{AsyncSignal, Config, State, SyncSignal, IO};
use eframe::egui;
use eyre::Result;
use itertools::Itertools;
use resource::{ResourceAddr, ResourceMap};
use std::any::Any;
use std::collections::BTreeSet;
use std::fmt::{Debug, Formatter};
use std::time::Instant;

pub trait Action: Debug + Any {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<ResourceAddr> {
        vec![]
    }

    #[inline(always)]
    fn init(self) -> Result<Box<dyn Action>>
    where
        Self: 'static + Sized,
    {
        Ok(Box::new(self))
    }

    fn stateful(
        &self,
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>>;
}

pub trait StatefulAction: Send {
    fn is_over(&self) -> Result<bool>;

    fn type_str(&self) -> String;

    fn props(&self) -> Props;

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal>;

    fn update(
        &mut self,
        _signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        Ok(Signal::none())
    }

    fn show(
        &mut self,
        _ui: &mut egui::Ui,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<()> {
        Ok(())
    }

    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal>;

    fn debug(&self) -> Vec<(&str, String)> {
        vec![
            ("type", format!("{:?}", self.type_str())),
            ("done", format!("{:?}", self.is_over())),
            ("viz", format!("{:?}", self.props().visual())),
            ("inf", format!("{:?}", self.props().infinite())),
        ]
    }
}

impl Debug for dyn StatefulAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Action({})",
            self.debug()
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .join(", ")
        )
    }
}

pub trait ImplStatefulAction: StatefulAction {}

#[derive(Debug, Clone)]
pub enum ActionSignal {
    UpdateGraph,
    KeyPress(Instant, BTreeSet<Key>),
    StateChanged(Instant, BTreeSet<SignalId>),
}
