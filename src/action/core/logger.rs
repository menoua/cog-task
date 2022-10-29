use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{IoManager, LoggerSignal, ResourceManager};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize, Serialize)]
pub struct Logger {
    group: String,
    in_mapping: BTreeMap<SignalId, String>,
}

stateful!(Logger {
    group: String,
    in_mapping: BTreeMap<SignalId, String>,
});

impl Action for Logger {
    #[inline]
    fn in_signals(&self) -> BTreeSet<SignalId> {
        self.in_mapping.keys().cloned().collect()
    }

    fn init(self) -> Result<Box<dyn Action>>
    where
        Self: 'static + Sized,
    {
        if self.group.is_empty() {
            Err(eyre!("Logger's `group` cannot be empty."))
        } else if self.in_mapping.is_empty() {
            Err(eyre!("Logger without `in_mapping` is useless."))
        } else {
            Ok(Box::new(self))
        }
    }

    fn stateful(
        &self,
        _io: &IoManager,
        _res: &ResourceManager,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Ok(Box::new(StatefulLogger {
            done: false,
            group: self.group.clone(),
            in_mapping: self.in_mapping.clone(),
        }))
    }
}

impl StatefulAction for StatefulLogger {
    impl_stateful!();

    fn props(&self) -> Props {
        INFINITE.into()
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        let mut entries = vec![];
        if let ActionSignal::StateChanged(_, signal) = signal {
            for id in signal {
                if let Some(name) = self.in_mapping.get(id) {
                    if let Some(value) = state.get(id) {
                        entries.push((name.clone(), value.clone()));
                    }
                }
            }
        }

        async_writer.push(LoggerSignal::Extend(self.group.clone(), entries));
        Ok(Signal::none())
    }
}
