use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE, VISUAL};
use crate::comm::{QWriter, SignalId};
use crate::resource::{ResourceAddr, ResourceMap};
use crate::server::{AsyncSignal, Config, State, SyncSignal, IO};
use eframe::egui;
use eyre::{eyre, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_cbor::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct Switch(SignalId, usize, Vec<Box<dyn Action>>);

enum Decision {
    Temporary(usize),
    Final(usize),
}

stateful!(Switch {
    control: SignalId,
    children: Vec<Box<dyn StatefulAction>>,
    decision: Decision,
});

impl Action for Switch {
    #[inline]
    fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
        self.2
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
            control: self.0,
            children: self
                .2
                .iter()
                .map(|a| {
                    a.stateful(io, res, config, sync_writer, async_writer)
                        .unwrap()
                })
                .collect(),
            decision: Decision::Temporary(self.1),
        }))
    }
}

impl StatefulAction for StatefulSwitch {
    impl_stateful!();

    #[inline]
    fn props(&self) -> Props {
        if let Decision::Final(i) = self.decision {
            self.children[i].props()
        } else {
            self.children
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
            Decision::Temporary(i) => i,
            _ => return Ok(()),
        };

        if !(0..self.children.len()).contains(&decision) {
            return Err(eyre!(
                "Switch ended up with a decision outside allowed boundary."
            ));
        }

        self.decision = Decision::Final(decision);
        self.children[decision].start(sync_writer, async_writer, state)
    }

    #[inline]
    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<()> {
        match self.decision {
            Decision::Temporary(_) => match signal {
                ActionSignal::Internal(_, signal) => {
                    if let SignalId::Internal(i) = self.control {
                        if let Some(Value::Integer(c)) = signal.get(&i) {
                            self.decision = Decision::Temporary(*c as usize);
                        }
                    }
                }
                ActionSignal::StateChanged => {
                    if let SignalId::State(i) = self.control {
                        if let Some(Value::Integer(c)) = state.get(&i) {
                            self.decision = Decision::Temporary(*c as usize);
                        }
                    }
                }
                _ => {}
            },
            Decision::Final(i) => {
                self.children[i].update(signal, sync_writer, async_writer, state)?;

                if self.children[i].is_over()? {
                    self.done = true;
                }
            }
        }

        Ok(())
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<()> {
        if let Decision::Final(i) = self.decision {
            self.children[i].show(ui, sync_writer, async_writer, state)
        } else {
            Ok(())
        }
    }

    #[inline]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<()> {
        if let Decision::Final(i) = self.decision {
            self.children[i].stop(sync_writer, async_writer, state)?;
        }
        self.done = true;
        Ok(())
    }
}
