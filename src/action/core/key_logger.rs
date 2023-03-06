use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{IoManager, LoggerSignal, OptionalString, ResourceManager};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use chrono::Local;
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::BTreeSet;
use std::time::Instant;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeyLogger {
    #[serde(default = "defaults::group")]
    group: OptionalString,
    #[serde(default)]
    out_key: SignalId,
}

stateful!(KeyLogger {
    group: Option<String>,
    out_key: SignalId,
});

mod defaults {
    use crate::resource::OptionalString;

    #[inline(always)]
    pub fn group() -> OptionalString {
        Some("keypress".to_owned()).into()
    }
}

impl Action for KeyLogger {
    fn out_signals(&self) -> BTreeSet<SignalId> {
        BTreeSet::from([self.out_key])
    }

    fn stateful(
        &self,
        _io: &IoManager,
        _res: &ResourceManager,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        if self.group.as_ref().is_none() && self.out_key == 0 {
            return Err(eyre!(
                "Both `group` and `out_key` for KeyLogger cannot be empty simultaneously."
            ));
        }

        let group = self.group.as_ref().map(|s| s.to_owned());

        Ok(Box::new(StatefulKeyLogger {
            done: false,
            group,
            out_key: self.out_key,
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
        if let Some(group) = self.group.as_ref() {
            async_writer.push(LoggerSignal::Append(
                group.clone(),
                ("event".to_owned(), Value::Text("start".to_owned())),
            ));
        }

        Ok(Signal::none())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        if let ActionSignal::KeyPress(_, keys) = signal {
            let entry = (
                "key".to_string(),
                Value::Array(keys.iter().map(|k| Value::Text(format!("{k:?}"))).collect()),
            );

            if self.out_key > 0 {
                sync_writer.push(SyncSignal::Emit(
                    Instant::now(),
                    Signal::from(
                        keys.iter()
                            .map(|k| (self.out_key, Value::Text(format!("{k:?}"))))
                            .collect::<Vec<_>>(),
                    ),
                ));
            }

            if let Some(group) = self.group.as_ref() {
                async_writer.push(AsyncSignal::Logger(
                    Local::now(),
                    LoggerSignal::Append(group.clone(), entry),
                ));
            }
        }

        Ok(Signal::none())
    }

    #[inline]
    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        if let Some(group) = self.group.as_ref() {
            async_writer.push(LoggerSignal::Append(
                group.clone(),
                ("event".to_owned(), Value::Text("stop".to_owned())),
            ));
        }
        Ok(Signal::none())
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("group", format!("{:?}", self.group))])
            .collect()
    }
}
