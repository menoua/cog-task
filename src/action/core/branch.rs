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
pub struct Branch {
    #[serde(default)]
    default: usize,
    children: Vec<Box<dyn Action>>,
    in_control: SignalId,
}

enum Decision {
    Temporary(usize),
    Final(usize),
}

stateful!(Branch {
    children: Vec<Box<dyn StatefulAction>>,
    in_control: SignalId,
    decision: Decision,
});

impl Action for Branch {
    #[inline]
    fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
        self.children
            .iter()
            .flat_map(|c| c.resources(config))
            .unique()
            .collect()
    }

    fn init(self) -> Result<Box<dyn Action>> {
        if self.children.is_empty() {
            Err(eyre!("Branch should have at least one child."))
        } else if self.default >= self.children.len() {
            Err(eyre!(
                "Branch default value should be less than the number of its children."
            ))
        } else {
            Ok(Box::new(self))
        }
    }

    fn stateful(
        &self,
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        let mut children = vec![];
        for c in self.children.iter() {
            children.push(c.stateful(io, res, config, sync_writer, async_writer)?);
        }

        Ok(Box::new(StatefulBranch {
            done: false,
            children,
            in_control: self.in_control,
            decision: Decision::Temporary(self.default),
        }))
    }
}

impl StatefulAction for StatefulBranch {
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
            Decision::Temporary(_) => {
                if let ActionSignal::StateChanged(_, signal) = signal {
                    if signal.contains(&self.in_control) {
                        match state.get(&self.in_control) {
                            Some(Value::Integer(i)) if *i < self.children.len() as i128 => {
                                self.decision = Decision::Temporary(*i as usize);
                            }
                            Some(Value::Integer(_)) => {
                                return Err(eyre!("Branch request is out of bounds."));
                            }
                            _ => {}
                        }
                    }
                }
            }
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
