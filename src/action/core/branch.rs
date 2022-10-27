use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE, VISUAL};
use crate::comm::{QWriter, SignalId};
use crate::resource::{ResourceAddr, ResourceMap};
use crate::server::{AsyncSignal, Config, State, SyncSignal, IO};
use crate::util::f64_as_i64;
use eframe::egui;
use eyre::{eyre, Context, Result};
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
        if matches!(self.decision, Decision::Final(_)) {
            return Ok(());
        }

        let decision = match state.get(&self.in_control) {
            Some(Value::Integer(i)) => {
                if *i < self.children.len() as i128 {
                    *i as usize
                } else {
                    return Err(eyre!("Branch request is out of bounds."));
                }
            }
            Some(Value::Float(x)) => {
                let x = f64_as_i64(*x).wrap_err("Non-integer number supplied to branch.")?;
                if (0..self.children.len() as i64).contains(&x) {
                    x as usize
                } else {
                    return Err(eyre!("Branch request is out of bounds."));
                }
            }
            None => {
                if let Decision::Temporary(i) = self.decision {
                    i
                } else {
                    return Ok(());
                }
            }
            _ => return Err(eyre!("Branch control is in invalid state.")),
        };

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
    ) -> Result<Vec<SyncSignal>> {
        if let Decision::Final(i) = self.decision {
            self.children[i].update(signal, sync_writer, async_writer, state)?;
            if self.children[i].is_over()? {
                self.done = true;
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
