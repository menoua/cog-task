use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT};
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::queue::QWriter;
use eframe::egui::Ui;
use serde::{Deserialize, Serialize};
use crate::scheduler::State;

#[derive(Debug, Deserialize, Serialize)]
pub struct Nil;

stateful!(Nil {});

impl Action for Nil {
    #[inline(always)]
    fn stateful(
        &self,
        _io: &IO,
        _res: &ResourceMap,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        Ok(Box::new(StatefulNil { done: false }))
    }
}

impl Nil {
    pub fn new() -> Self {
        Self
    }
}

impl StatefulAction for StatefulNil {
    impl_stateful!();

    fn props(&self) -> Props {
        DEFAULT.into()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        self.done = true;
        sync_writer.push(SyncSignal::UpdateGraph);
        Ok(())
    }

    fn update(
        &mut self,
        _signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn show(
        &mut self,
        _ui: &mut Ui,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        Ok(())
    }
}

impl StatefulNil {
    pub fn new() -> Self {
        Self { done: true }
    }
}
