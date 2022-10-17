use crate::action::{Action, ActionSignal, CAP_KEYS, DEFAULT, Props, StatefulAction, ActionEnum, StatefulActionEnum};
use crate::signal::QWriter;
use crate::config::Config;
use crate::error;
use crate::error::Error::InvalidNameError;
use crate::io::IO;
use crate::logger::LoggerSignal;
use crate::resource::ResourceMap;
use crate::scheduler::monitor::{Event, Monitor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeyLogger {
    #[serde(default = "defaults::group")]
    group: String,
}

stateful!(KeyLogger { group: String });

mod defaults {
    #[inline(always)]
    pub fn group() -> String {
        "keypress".to_owned()
    }
}

impl Action for KeyLogger {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![]
    }

    fn stateful(
        &self,
        id: usize,
        _res: &ResourceMap,
        _config: &Config,
        _io: &IO,
    ) -> Result<StatefulActionEnum, error::Error> {
        if self.group.is_empty() {
            Err(InvalidNameError(
                "KeyLogger `group` cannot be an empty string".to_owned(),
            ))
        } else {
            Ok(StatefulKeyLogger {
                id,
                done: false,
                group: self.group.clone(),
            }.into())
        }
    }
}

impl StatefulAction for StatefulKeyLogger {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        CAP_KEYS.into()
    }

    fn update(
        &mut self,
        signal: ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        if let ActionSignal::KeyPress(keys) = signal {
            let group = self.group.clone();
            let entry = (
                "key".to_string(),
                Value::Array(
                    keys.into_iter()
                        .map(|k| Value::String(format!("{k:?}")))
                        .collect(),
                ),
            );
            async_writer.push(LoggerSignal::Append(group, entry));
        }
        Ok(())
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
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
