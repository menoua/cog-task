use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE, VISUAL};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{ResourceAddr, ResourceMap, IO};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use crate::util::f64_as_i64;
use eframe::egui;
use eyre::{eyre, Context, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize, Serialize)]
pub struct View {
    #[serde(default)]
    default: usize,
    children: Vec<Box<dyn Action>>,
    in_control: SignalId,
}

stateful!(View {
    children: Vec<Box<dyn StatefulAction>>,
    in_control: SignalId,
    choice: usize,
});

impl Action for View {
    #[inline]
    fn in_signals(&self) -> BTreeSet<SignalId> {
        let mut signals = BTreeSet::from([self.in_control]);
        for c in self.children.iter() {
            signals.extend(c.in_signals());
        }
        signals
    }

    #[inline]
    fn out_signals(&self) -> BTreeSet<SignalId> {
        let mut signals = BTreeSet::new();
        for c in self.children.iter() {
            signals.extend(c.out_signals());
        }
        signals
    }

    #[inline]
    fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
        self.children
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
        let mut children = vec![];
        for c in self.children.iter() {
            children.push(c.stateful(io, res, config, sync_writer, async_writer)?);
        }

        Ok(Box::new(StatefulView {
            done: false,
            children,
            in_control: self.in_control,
            choice: self.default,
        }))
    }
}

impl StatefulAction for StatefulView {
    impl_stateful!();

    #[inline]
    fn props(&self) -> Props {
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

    #[inline]
    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        self.choice = match state.get(&self.in_control) {
            Some(Value::Integer(i)) => {
                if *i < self.children.len() as i128 {
                    *i as usize
                } else {
                    return Err(eyre!("Branch request is out of bounds."));
                }
            }
            Some(Value::Float(x)) => {
                let x = f64_as_i64(*x).wrap_err("Non-integer number supplied to view.")?;
                if (0..self.children.len() as i64).contains(&x) {
                    x as usize
                } else {
                    return Err(eyre!("Branch request is out of bounds."));
                }
            }
            None => self.choice,
            _ => return Err(eyre!("View control is in invalid state.")),
        };

        let mut news = vec![];
        for c in self.children.iter_mut() {
            news.extend(c.start(sync_writer, async_writer, state)?);
        }

        Ok(news.into())
    }

    #[inline]
    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        if let ActionSignal::StateChanged(_, signal) = signal {
            if signal.contains(&self.in_control) {
                self.choice = match state.get(&self.in_control) {
                    Some(Value::Integer(i)) => {
                        if *i < self.children.len() as i128 {
                            *i as usize
                        } else {
                            return Err(eyre!("View request is out of bounds."));
                        }
                    }
                    Some(Value::Float(x)) => {
                        let x = f64_as_i64(*x).wrap_err("Non-integer number supplied to view.")?;
                        if (0..self.children.len() as i64).contains(&x) {
                            x as usize
                        } else {
                            return Err(eyre!("View request is out of bounds."));
                        }
                    }
                    None => self.choice,
                    _ => return Err(eyre!("View control is in invalid state.")),
                };
            }
        }

        let mut news = vec![];
        for c in self.children.iter_mut() {
            news.extend(c.update(signal, sync_writer, async_writer, state)?);
        }

        if self.children[self.choice].is_over()? {
            self.done = true;
        }

        Ok(news.into())
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<()> {
        self.children[self.choice].show(ui, sync_writer, async_writer, state)
    }

    #[inline]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        self.done = true;
        let mut news = vec![];
        for c in self.children.iter_mut() {
            news.extend(c.stop(sync_writer, async_writer, state)?);
        }
        Ok(news.into())
    }
}
