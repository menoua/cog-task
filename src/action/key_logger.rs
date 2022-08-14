use crate::action::{Action, StatefulAction, StatefulActionMsg};
use crate::config::Config;
use crate::error;
use crate::error::Error::InvalidNameError;
use crate::io::IO;
use crate::logger::LoggerMsg;
use crate::resource::ResourceMap;
use crate::scheduler::monitor::{Event, Monitor};
use crate::server::ServerMsg;
use iced::Command;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeyLogger {
    #[serde(default = "defaults::group")]
    group: String,
}

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
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        if self.group.is_empty() {
            Err(InvalidNameError(
                "KeyLogger `group` cannot be an empty string".to_owned(),
            ))
        } else {
            Ok(Box::new(StatefulKeyLogger {
                id,
                done: false,
                group: self.group.clone(),
            }))
        }
    }
}

#[derive(Debug)]
pub struct StatefulKeyLogger {
    id: usize,
    done: bool,
    group: String,
}

impl StatefulAction for StatefulKeyLogger {
    #[inline(always)]
    fn id(&self) -> usize {
        self.id
    }

    #[inline(always)]
    fn is_over(&self) -> Result<bool, error::Error> {
        Ok(self.done)
    }

    #[inline(always)]
    fn is_visual(&self) -> bool {
        false
    }

    #[inline(always)]
    fn is_static(&self) -> bool {
        true
    }

    #[inline(always)]
    fn monitors(&self) -> Option<Monitor> {
        Some(Monitor::Keys)
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        self.done = true;
        Ok(())
    }

    fn update(&mut self, msg: StatefulActionMsg) -> Result<Command<ServerMsg>, error::Error> {
        if let StatefulActionMsg::UpdateEvent(Event::Key(key)) = msg {
            let group = self.group.clone();
            let entry = ("key".to_string(), Value::String(format!("{key:?}")));
            Ok(Command::perform(
                async move { LoggerMsg::Append(group, entry) },
                LoggerMsg::wrap,
            ))
        } else {
            Ok(Command::none())
        }
    }
}
