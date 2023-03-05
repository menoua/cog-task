use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{IoManager, ResourceAddr, ResourceManager};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eframe::egui;
use eframe::egui::Response;
use eyre::{eyre, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

#[derive(Debug, Deserialize, Serialize)]
pub struct Seq(Vec<Box<dyn Action>>);

stateful!(Seq {
    children: VecDeque<Box<dyn StatefulAction>>,
});

impl Seq {
    pub fn new(children: Vec<Box<dyn Action>>) -> Self {
        Self(children)
    }
}

impl Action for Seq {
    #[inline]
    fn in_signals(&self) -> BTreeSet<SignalId> {
        let mut signals = BTreeSet::new();
        for c in self.0.iter() {
            signals.extend(c.in_signals());
        }
        signals
    }

    #[inline]
    fn out_signals(&self) -> BTreeSet<SignalId> {
        let mut signals = BTreeSet::new();
        for c in self.0.iter() {
            signals.extend(c.out_signals());
        }
        signals
    }

    #[inline]
    fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
        self.0
            .iter()
            .flat_map(|c| c.resources(config))
            .unique()
            .collect()
    }

    fn stateful(
        &self,
        io: &IoManager,
        res: &ResourceManager,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        let mut children: VecDeque<_> = VecDeque::with_capacity(self.0.len());
        for c in self.0.iter() {
            children.push_back(c.stateful(io, res, config, sync_writer, async_writer)?);
        }

        for c in children.iter().take(children.len() - 1) {
            if c.props().infinite() {
                return Err(eyre!("Only the final action in a `Seq` can be infinite."));
            }
        }

        Ok(Box::new(StatefulSeq {
            done: false,
            children,
        }))
    }
}

impl StatefulSeq {
    pub fn push(&mut self, child: impl Into<Box<dyn StatefulAction>>) {
        self.children.push_back(child.into());
    }
}

impl StatefulAction for StatefulSeq {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        if let Some(c) = self.children.get(0) {
            c.props()
        } else {
            DEFAULT.into()
        }
    }

    #[inline]
    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        if let Some(c) = self.children.get_mut(0) {
            c.start(sync_writer, async_writer, state)
        } else {
            self.done = true;
            sync_writer.push(SyncSignal::UpdateGraph);
            Ok(Signal::none())
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
        if let Some(c) = self.children.get_mut(0) {
            let mut news = vec![];
            news.extend(c.update(signal, sync_writer, async_writer, state)?);

            if c.is_over()? {
                news.extend(self.children.pop_front().unwrap().stop(
                    sync_writer,
                    async_writer,
                    state,
                )?);

                if let Some(c) = self.children.get_mut(0) {
                    news.extend(c.start(sync_writer, async_writer, state)?);
                } else {
                    self.done = true;
                }
            }

            Ok(news.into())
        } else {
            Ok(Signal::none())
        }
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Response> {
        if let Some(c) = self.children.get_mut(0) {
            c.show(ui, sync_writer, async_writer, state)
        } else {
            Ok(ui.label(""))
        }
    }

    #[inline]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        if let Some(c) = self.children.get_mut(0) {
            c.stop(sync_writer, async_writer, state)
        } else {
            Ok(Signal::none())
        }
    }
}
