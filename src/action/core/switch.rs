use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE, VISUAL};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{ResourceAddr, ResourceMap};
use crate::server::{AsyncSignal, Config, State, SyncSignal, IO};
use crate::util::approx_eq;
use eframe::egui;
use eyre::{eyre, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_cbor::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct Switch {
    #[serde(default)]
    default: bool,
    #[serde(alias = "if")]
    if_true: Box<dyn Action>,
    #[serde(alias = "else")]
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
    ) -> Result<Signal> {
        if matches!(self.decision, Decision::Final(_)) {
            return Err(eyre!("Tried to restart a switch."));
        }

        let decision = match state.get(&self.in_control) {
            Some(Value::Bool(c)) => *c,
            Some(Value::Integer(1)) => true,
            Some(Value::Integer(0)) => false,
            Some(Value::Float(x)) if approx_eq(*x, 1.0) => true,
            Some(Value::Float(x)) if approx_eq(*x, 0.0) => false,
            Some(v) => return Err(eyre!("Failed to interpret value ({v:?}) as boolean.")),
            None => return Err(eyre!("Control state is missing.")),
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
    ) -> Result<Signal> {
        match self.decision {
            Decision::Final(true) => {
                let news = self
                    .if_true
                    .update(signal, sync_writer, async_writer, state)?;
                if self.if_true.is_over()? {
                    self.done = true;
                }
                Ok(news)
            }
            Decision::Final(false) => {
                let news = self
                    .if_false
                    .update(signal, sync_writer, async_writer, state)?;
                if self.if_false.is_over()? {
                    self.done = true;
                }
                Ok(news)
            }
            _ => Ok(Signal::none()),
        }
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
    ) -> Result<Signal> {
        self.done = true;
        match self.decision {
            Decision::Final(true) => self.if_true.stop(sync_writer, async_writer, state),
            Decision::Final(false) => self.if_true.stop(sync_writer, async_writer, state),
            Decision::Temporary(_) => Ok(Signal::none()),
        }
    }
}
