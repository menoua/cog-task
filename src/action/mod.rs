#[macro_use]
mod macros;
pub mod core;
pub mod de;
pub mod extra;
pub mod include;
pub mod props;

use crate::config::Config;
use crate::error;
use crate::io::IO;
use crate::queue::QWriter;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::scheduler::State;
use crate::signal::IntSignal;
use eframe::egui;
use itertools::Itertools;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::time::Instant;

pub use include::*;
pub use props::*;

pub trait Action: Debug {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![]
    }

    #[inline(always)]
    fn init(self) -> Result<Box<dyn Action>, error::Error>
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
    ) -> Result<Box<dyn StatefulAction>, error::Error>;
}

pub trait StatefulAction: Send {
    fn is_over(&self) -> Result<bool, error::Error>;

    fn type_str(&self) -> String;

    fn props(&self) -> Props;

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<(), error::Error>;

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<(), error::Error>;

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<(), error::Error>;

    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<(), error::Error>;

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
    KeyPress(Instant, HashSet<egui::Key>),
    Internal(Instant, IntSignal),
    // External(Instant, ExtSignal),
    StateChanged,
}
