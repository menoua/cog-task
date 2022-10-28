use crate::action::{Action, Props, StatefulAction, DEFAULT};
use crate::comm::{QWriter, Signal};
use crate::resource::{ResourceMap, IO};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eyre::Result;
use serde::{Deserialize, Serialize};

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
    ) -> Result<Box<dyn StatefulAction>> {
        Ok(Box::new(StatefulNil { done: false }))
    }
}

impl Nil {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Nil {
    fn default() -> Self {
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
    ) -> Result<Signal> {
        self.done = true;
        sync_writer.push(SyncSignal::UpdateGraph);
        Ok(Signal::none())
    }

    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        self.done = true;
        Ok(Signal::none())
    }
}

impl StatefulNil {
    pub fn new() -> Self {
        Self { done: false }
    }
}

impl Default for StatefulNil {
    fn default() -> Self {
        Self::new()
    }
}
