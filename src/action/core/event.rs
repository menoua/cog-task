use crate::action::{Action, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, Signal};
use crate::resource::{LoggerSignal, ResourceMap, IO};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_cbor::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct Event(String);

stateful!(Event { name: String });

impl Action for Event {
    #[inline(always)]
    fn stateful(
        &self,
        _io: &IO,
        _res: &ResourceMap,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Ok(Box::new(StatefulEvent {
            done: false,
            name: self.0.clone(),
        }))
    }
}

impl StatefulAction for StatefulEvent {
    impl_stateful!();

    fn props(&self) -> Props {
        INFINITE.into()
    }

    fn start(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        async_writer.push(LoggerSignal::Append(
            "event".to_owned(),
            (self.name.clone(), Value::Text("start".to_owned())),
        ));
        Ok(Signal::none())
    }

    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        async_writer.push(LoggerSignal::Append(
            "event".to_owned(),
            (self.name.clone(), Value::Text("stop".to_owned())),
        ));

        self.done = true;
        Ok(Signal::none())
    }
}
