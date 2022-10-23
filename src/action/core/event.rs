use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::io::IO;
use crate::logger::LoggerSignal;
use crate::queue::QWriter;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::scheduler::State;
use eframe::egui::Ui;
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
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
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
    ) -> Result<(), Error> {
        async_writer.push(LoggerSignal::Append(
            "event".to_owned(),
            (self.name.clone(), Value::Text("start".to_owned())),
        ));
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
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        async_writer.push(LoggerSignal::Append(
            "event".to_owned(),
            (self.name.clone(), Value::Text("stop".to_owned())),
        ));

        self.done = true;
        Ok(())
    }
}
