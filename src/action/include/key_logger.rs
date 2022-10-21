use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::error::Error::InvalidNameError;
use crate::io::IO;
use crate::logger::LoggerSignal;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use eframe::egui::Ui;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeyLogger(#[serde(default = "defaults::group")] String);

stateful!(KeyLogger { group: String });

mod defaults {
    #[inline]
    pub fn group() -> String {
        "keypress".to_owned()
    }
}

impl Action for KeyLogger {
    #[inline]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![]
    }

    fn stateful(
        &self,
        _io: &IO,
        _res: &ResourceMap,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        if self.0.is_empty() {
            return Err(InvalidNameError(
                "KeyLogger `group` cannot be an empty string".to_owned(),
            ));
        }

        Ok(Box::new(StatefulKeyLogger {
            done: false,
            group: self.0.clone(),
        }))
    }
}

impl StatefulAction for StatefulKeyLogger {
    impl_stateful!();

    #[inline]
    fn props(&self) -> Props {
        INFINITE.into()
    }

    fn start(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        async_writer.push(LoggerSignal::Append(
            self.group.clone(),
            ("event".to_owned(), Value::String("start".to_owned())),
        ));
        Ok(())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        if let ActionSignal::KeyPress(keys) = signal {
            let group = self.group.clone();
            let entry = (
                "key".to_string(),
                Value::Array(
                    keys.iter()
                        .map(|k| Value::String(format!("{k:?}")))
                        .collect(),
                ),
            );
            async_writer.push(LoggerSignal::Append(group, entry));
        }
        Ok(())
    }

    fn show(
        &mut self,
        _ui: &mut Ui,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        Ok(())
    }

    #[inline]
    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        async_writer.push(LoggerSignal::Append(
            self.group.clone(),
            ("event".to_owned(), Value::String("stop".to_owned())),
        ));

        self.done = true;
        Ok(())
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("group", format!("{:?}", self.group))])
            .collect()
    }
}
