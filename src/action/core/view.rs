use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE, VISUAL};
use crate::config::Config;
use crate::error;
use crate::io::IO;
use crate::queue::QWriter;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::scheduler::State;
use crate::signal::SignalId;
use eframe::egui;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct View(SignalId, usize, Vec<Box<dyn Action>>);

stateful!(View {
    control: SignalId,
    children: Vec<Box<dyn StatefulAction>>,
    decision: usize,
});

impl Action for View {
    #[inline]
    fn resources(&self, config: &Config) -> Vec<PathBuf> {
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
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        Ok(Box::new(StatefulView {
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
            decision: self.1,
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
    ) -> Result<(), error::Error> {
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
    ) -> Result<(), error::Error> {
        match signal {
            ActionSignal::Internal(_, signal) => {
                if let SignalId::Internal(i) = self.control {
                    if let Some(Value::Integer(c)) = signal.get(&i) {
                        self.decision = *c as usize;
                    }
                }
            }
            ActionSignal::StateChanged => {
                if let SignalId::State(i) = self.control {
                    if let Some(Value::Integer(c)) = state.get(&i) {
                        self.decision = *c as usize;
                    }
                }
            }
            _ => {}
        }

        for c in self.children.iter_mut() {
            c.update(signal, sync_writer, async_writer, state)?;
        }

        if self.children[self.decision].is_over()? {
            self.done = true;
        }

        Ok(())
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<(), error::Error> {
        self.children[self.decision].show(ui, sync_writer, async_writer, state)
    }

    #[inline]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<(), error::Error> {
        for c in self.children.iter_mut() {
            c.stop(sync_writer, async_writer, state)?;
        }
        self.done = true;
        Ok(())
    }
}
