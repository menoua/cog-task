use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, Signal};
use crate::resource::{IoManager, LoggerSignal, ResourceManager};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use chrono::Local;
use eframe::egui::Ui;
use eyre::{eyre, Error, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeyLogger(#[serde(default = "defaults::group")] String);

stateful!(KeyLogger { group: String });

mod defaults {
    #[inline(always)]
    pub fn group() -> String {
        "keypress".to_owned()
    }
}

impl Action for KeyLogger {
    fn stateful(
        &self,
        _io: &IoManager,
        _res: &ResourceManager,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        if self.0.is_empty() {
            return Err(eyre!("KeyLogger `group` cannot be an empty string"));
        }

        Ok(Box::new(StatefulKeyLogger {
            done: false,
            group: self.0.clone(),
        }))
    }
}

impl StatefulAction for StatefulKeyLogger {
    impl_stateful!();

    #[inline(always)]
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
            self.group.clone(),
            ("event".to_owned(), Value::Text("start".to_owned())),
        ));
        Ok(Signal::none())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        if let ActionSignal::KeyPress(_, keys) = signal {
            let group = self.group.clone();
            let entry = (
                "key".to_string(),
                Value::Array(keys.iter().map(|k| Value::Text(format!("{k:?}"))).collect()),
            );

            async_writer.push(AsyncSignal::Logger(
                Local::now(),
                LoggerSignal::Append(group, entry),
            ));
        }

        Ok(Signal::none())
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

    #[inline]
    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        async_writer.push(LoggerSignal::Append(
            self.group.clone(),
            ("event".to_owned(), Value::Text("stop".to_owned())),
        ));

        self.done = true;
        Ok(Signal::none())
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("group", format!("{:?}", self.group))])
            .collect()
    }
}
