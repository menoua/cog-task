use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{IoManager, ResourceAddr, ResourceManager};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eframe::egui::{Response, Ui};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize, Serialize)]
pub struct Until {
    inner: Box<dyn Action>,
    #[serde(default)]
    in_condition: SignalId,
    #[serde(default)]
    in_event: SignalId,
}

stateful!(Until {
    inner: Box<dyn StatefulAction>,
    in_condition: SignalId,
    in_event: SignalId,
});

impl Action for Until {
    fn init(self) -> Result<Box<dyn Action>>
    where
        Self: 'static + Sized,
    {
        if self.in_event == 0 && self.in_condition == 0 {
            Err(eyre!(
                "At least one of `in_event` and ``in_condition` have to be set for Until."
            ))
        } else {
            Ok(Box::new(self))
        }
    }

    fn in_signals(&self) -> BTreeSet<SignalId> {
        let mut signals = BTreeSet::from([self.in_event, self.in_condition]);
        signals.extend(self.inner.out_signals());
        signals
    }

    fn out_signals(&self) -> BTreeSet<SignalId> {
        self.inner.out_signals()
    }

    fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
        self.inner.resources(config)
    }

    fn stateful(
        &self,
        io: &IoManager,
        res: &ResourceManager,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Ok(Box::new(StatefulUntil {
            done: false,
            inner: self
                .inner
                .stateful(io, res, config, sync_writer, async_writer)?,
            in_condition: self.in_condition,
            in_event: self.in_event,
        }))
    }
}

impl StatefulAction for StatefulUntil {
    impl_stateful!();

    fn props(&self) -> Props {
        (self.inner.props().bits() & !INFINITE).into()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        let mut done = false;
        if let Some(&Value::Bool(true)) = state.get(&self.in_condition) {
            done = true;
        }
        if state.contains_key(&self.in_event) {
            done = true;
        }

        if done {
            self.done = true;
            sync_writer.push(SyncSignal::UpdateGraph);
            Ok(Signal::none())
        } else {
            self.inner.start(sync_writer, async_writer, state)
        }
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        let mut done = false;
        if let ActionSignal::StateChanged(_, signal) = signal {
            for id in signal {
                if id == &self.in_event {
                    done = true;
                } else if id == &self.in_condition {
                    done |= match state.get(&self.in_condition) {
                        None => false,
                        Some(Value::Bool(c)) => *c,
                        Some(Value::Integer(i)) => *i > 0,
                        Some(Value::Float(f)) => *f > 0.0,
                        Some(v) => {
                            return Err(eyre!(
                                "Invalid value ({v:?}) supplied as condition for Until."
                            ))
                        }
                    };
                }
            }
        }

        if done {
            self.done = true;
            sync_writer.push(SyncSignal::UpdateGraph);
            Ok(Signal::none())
        } else {
            self.inner.update(signal, sync_writer, async_writer, state)
        }
    }

    fn show(
        &mut self,
        ui: &mut Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Response> {
        self.inner.show(ui, sync_writer, async_writer, state)
    }

    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        self.inner.stop(sync_writer, async_writer, state)
    }
}
