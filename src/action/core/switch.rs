use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE, VISUAL};
use crate::comm::{QWriter, SignalId};
use crate::resource::{ResourceAddr, ResourceMap};
use crate::server::{AsyncSignal, Config, State, SyncSignal, IO};
use eframe::egui;
use eyre::Result;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_cbor::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct Switch {
    #[serde(default)]
    default: bool,
    #[serde(rename = "true")]
    if_true: Box<dyn Action>,
    #[serde(rename = "false")]
    if_false: Box<dyn Action>,
    in_control: SignalId,
}

enum Decision {
    Temporary(bool),
    Final(bool),
}

stateful!(Switch {
    if_true: Box<dyn StatefulAction>,
    if_false: Box<dyn StatefulAction>,
    in_control: SignalId,
    decision: Decision,
});

impl Action for Switch {
    #[inline]
    fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
        [&self.if_true, &self.if_false]
            .iter()
            .flat_map(|c| c.resources(config))
            .unique()
            .collect()
    }

    fn stateful(
        &self,
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Ok(Box::new(StatefulSwitch {
            done: false,
            if_true: self
                .if_true
                .stateful(io, res, config, sync_writer, async_writer)?,
            if_false: self
                .if_false
                .stateful(io, res, config, sync_writer, async_writer)?,
            in_control: self.in_control,
            decision: Decision::Temporary(self.default),
        }))
    }
}

impl StatefulAction for StatefulSwitch {
    impl_stateful!();

    #[inline]
    fn props(&self) -> Props {
        if let Decision::Final(true) = self.decision {
            self.if_true.props()
        } else if let Decision::Final(false) = self.decision {
            self.if_false.props()
        } else {
            [&self.if_true, &self.if_false]
                .iter()
                .fold(DEFAULT, |mut state, c| {
                    let c = c.props();
                    if c.visual() {
                        state |= VISUAL;
                    }
                    if c.infinite() {
                        state |= INFINITE;
                    }
                    state
                })
                .into()
        }
    }

    #[inline]
    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<()> {
        let decision = match self.decision {
            Decision::Temporary(c) => c,
            _ => return Ok(()),
        };

        self.decision = Decision::Final(decision);
        if decision {
            self.if_true.start(sync_writer, async_writer, state)
        } else {
            self.if_false.start(sync_writer, async_writer, state)
        }
    }

    #[inline]
    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Vec<SyncSignal>> {
        match self.decision {
            Decision::Temporary(_) => {
                if let ActionSignal::StateChanged(_, signal) = signal {
                    if signal.contains(&self.in_control) {
                        if let Some(Value::Bool(c)) = state.get(&self.in_control) {
                            self.decision = Decision::Temporary(*c);
                        }
                    }
                }
            }
            Decision::Final(true) => {
                self.if_true
                    .update(signal, sync_writer, async_writer, state)?;
                if self.if_true.is_over()? {
                    self.done = true;
                }
            }
            Decision::Final(false) => {
                self.if_false
                    .update(signal, sync_writer, async_writer, state)?;
                if self.if_false.is_over()? {
                    self.done = true;
                }
            }
        }

        Ok(vec![])
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<()> {
        match self.decision {
            Decision::Final(true) => self.if_true.show(ui, sync_writer, async_writer, state),
            Decision::Final(false) => self.if_false.show(ui, sync_writer, async_writer, state),
            Decision::Temporary(_) => Ok(()),
        }
    }

    #[inline]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<()> {
        match self.decision {
            Decision::Final(true) => self.if_true.stop(sync_writer, async_writer, state)?,
            Decision::Final(false) => self.if_true.stop(sync_writer, async_writer, state)?,
            Decision::Temporary(_) => {}
        }

        self.done = true;
        Ok(())
    }
}
