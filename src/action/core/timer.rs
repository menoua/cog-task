use crate::action::{Action, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::ResourceMap;
use crate::server::{AsyncSignal, Config, LoggerSignal, State, SyncSignal, IO};
use eyre::{eyre, Error, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::BTreeSet;
use std::time::Instant;

#[derive(Debug, Deserialize, Serialize)]
pub struct Timer {
    #[serde(default)]
    name: String,
    #[serde(default)]
    out_duration: SignalId,
}

stateful!(Timer {
    name: String,
    out_duration: SignalId,
    since: Instant,
});

impl Action for Timer {
    #[inline]
    fn out_signals(&self) -> BTreeSet<SignalId> {
        BTreeSet::from([self.out_duration])
    }

    fn init(self) -> Result<Box<dyn Action>>
    where
        Self: 'static + Sized,
    {
        if self.name.is_empty() && self.out_duration == 0 {
            Err(eyre!(
                "`Timer` without a `name` or `out_duration` is useless."
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
            out_duration: self.out_duration,
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
    ) -> Result<Signal, Error> {
        self.since = Instant::now();
        Ok(Signal::none())
    }

    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        let duration = Instant::now() - self.since;
        if !self.name.is_empty() {
            async_writer.push(LoggerSignal::Append(
                "timer".to_owned(),
                (self.name.clone(), Value::Text(format!("{duration:?}"))),
            ));
        }

        self.done = true;
        if self.out_duration > 0 {
            Ok(vec![(self.out_duration, Value::Float(duration.as_secs_f64()))].into())
        } else {
            Ok(Signal::none())
        }
    }
}
