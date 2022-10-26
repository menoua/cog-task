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
pub struct View {
    #[serde(default)]
    default: usize,
    children: Vec<Box<dyn Action>>,
    in_control: SignalId,
}

stateful!(View {
    children: Vec<Box<dyn StatefulAction>>,
    in_control: SignalId,
    decision: usize,
});

impl Action for View {
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
            decision: self.default,
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
    ) -> Result<()> {
        for c in self.children.iter_mut() {
            c.start(sync_writer, async_writer, state)?;
        }

        Ok(())
    }

    #[inline]
    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Vec<SyncSignal>> {
        if let ActionSignal::StateChanged(_, signal) = signal {
            if signal.contains(&self.in_control) {
                match state.get(&self.in_control) {
                    Some(Value::Integer(i)) if *i < self.children.len() as i128 => {
                        self.decision = *i as usize;
                    }
                    Some(Value::Integer(_)) => return Err(eyre!("View request is out of bounds.")),
                    _ => {}
                }
            }
        }

        for c in self.children.iter_mut() {
            c.update(signal, sync_writer, async_writer, state)?;
        }

        if self.children[self.decision].is_over()? {
            self.done = true;
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
        self.children[self.decision].show(ui, sync_writer, async_writer, state)
    }

    #[inline]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<()> {
        for c in self.children.iter_mut() {
            c.stop(sync_writer, async_writer, state)?;
        }
        self.done = true;
        Ok(())
    }
}
