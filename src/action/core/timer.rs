use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, SignalId};
use crate::resource::ResourceMap;
use crate::server::{AsyncSignal, Config, LoggerSignal, State, SyncSignal, IO};
use eframe::egui::Ui;
use eyre::{eyre, Error, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::time::Instant;

#[derive(Debug, Deserialize, Serialize)]
pub struct Timer {
    #[serde(default)]
    name: String,
    #[serde(default)]
    sig_duration: SignalId,
}

stateful!(Timer {
    name: String,
    sig_duration: SignalId,
    since: Instant,
});

impl Action for Timer {
    fn init(self) -> Result<Box<dyn Action>>
    where
        Self: 'static + Sized,
    {
        if self.name.is_empty() && self.sig_duration.is_none() {
            Err(eyre!(
                "`Timer` without a `name` or `sig_duration` is useless."
            ))
        } else {
            Ok(Box::new(self))
        }
    }

    #[inline(always)]
    fn stateful(
        &self,
        _io: &IO,
        _res: &ResourceMap,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Ok(Box::new(StatefulTimer {
            done: false,
            name: self.name.clone(),
            sig_duration: self.sig_duration,
            since: Instant::now(),
        }))
    }
}

impl StatefulAction for StatefulTimer {
    impl_stateful!();

    fn props(&self) -> Props {
        INFINITE.into()
    }

    fn start(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        self.since = Instant::now();
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
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        let duration = Instant::now() - self.since;
        if !self.name.is_empty() {
            async_writer.push(LoggerSignal::Append(
                "timer".to_owned(),
                (self.name.clone(), Value::Text(format!("{duration:?}"))),
            ));
        }

        if !self.sig_duration.is_none() {
            sync_writer.push(SyncSignal::Emit(
                Instant::now(),
                vec![(self.sig_duration, Value::Float(duration.as_secs_f64()))].into(),
            ))
        }

        self.done = true;
        Ok(())
    }
}
