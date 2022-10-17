use crate::config::{Config, LogCondition};
use crate::error;
use crate::error::Error::{ActionViewError, InvalidNameError};
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::monitor::{Event, Monitor};
use eframe::egui;
use itertools::Itertools;
use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};

#[macro_use]
mod macros;
pub mod include;
pub mod props;

use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
pub use include::*;
pub use props::*;

pub trait Action: Debug {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![]
    }

    #[inline(always)]
    fn init(&self) -> Result<(), error::Error> {
        Ok(())
    }

    fn stateful(
        &self,
        id: usize,
        res: &ResourceMap,
        config: &Config,
        io: &IO,
    ) -> Result<StatefulActionEnum, error::Error>;

    #[inline(always)]
    fn evolve(
        &self,
        _root_dir: &Path,
        _config: &Config,
    ) -> Result<Option<ActionEnum>, error::Error> {
        Ok(None)
    }
}

pub trait StatefulAction: Send {
    fn id(&self) -> usize;

    fn is_over(&self) -> Result<bool, error::Error>;

    fn type_str(&self) -> String;

    fn props(&self) -> Props;

    #[inline(always)]
    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        Ok(())
    }

    #[inline(always)]
    fn update(
        &mut self,
        signal: ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        Ok(())
    }

    #[inline(always)]
    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        Err(ActionViewError(format!(
            "Attempted to show a non-visual action: Action({})",
            self.debug()
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .join(", ")
        )))
    }

    fn stop(&mut self) -> Result<(), error::Error>;

    fn debug(&self) -> Vec<(&str, String)> {
        vec![
            ("id", format!("{:?}", self.id())),
            ("over", format!("{:?}", self.is_over())),
            ("visual", format!("{:?}", self.props().visual())),
            ("finite", format!("{:?}", self.props().finite())),
            ("key_cap", format!("{:?}", self.props().captures_keys())),
            ("type", format!("{:?}", self.type_str())),
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
    KeyPress(HashSet<egui::Key>),
}
