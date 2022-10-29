#[macro_use]
mod macros;
pub mod core;
pub mod de;
pub mod extra;
pub mod include;
pub mod props;

pub use include::*;
pub use props::*;

use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{IoManager, Key, ResourceAddr, ResourceManager};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eframe::egui;
use eyre::Result;
use itertools::Itertools;
use std::any::Any;
use std::collections::BTreeSet;
use std::fmt::{Debug, Formatter};
use std::time::Instant;

pub trait Action: Debug + Any {
    #[inline(always)]
    fn init(self) -> Result<Box<dyn Action>>
    where
        Self: 'static + Sized,
    {
        Ok(Box::new(self))
    }

    #[inline(always)]
    fn in_signals(&self) -> BTreeSet<SignalId> {
        BTreeSet::new()
    }

    #[inline(always)]
    fn out_signals(&self) -> BTreeSet<SignalId> {
        BTreeSet::new()
    }

    #[inline(always)]
    #[allow(unused_variables)]
    fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
        vec![]
    }

    fn stateful(
        &self,
        io: &IoManager,
        res: &ResourceManager,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>>;
}

pub trait StatefulAction: Send {
    fn is_over(&self) -> Result<bool>;

    fn type_str(&self) -> String;

    fn props(&self) -> Props {
        DEFAULT.into()
    }

    #[allow(unused_variables)]
    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        if self.props().visual() {
            sync_writer.push(SyncSignal::Repaint);
        }
        Ok(Signal::none())
    }

    #[allow(unused_variables)]
    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        Ok(Signal::none())
    }

    #[allow(unused_variables)]
    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        if self.props().visual() {
            sync_writer.push(SyncSignal::Repaint);
        }
        Ok(Signal::none())
    }

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
