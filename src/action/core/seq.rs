use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT};
use crate::config::Config;
use crate::io::IO;
use crate::queue::QWriter;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::scheduler::State;
use eframe::egui;
use eyre::{eyre, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;

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
    fn resources(&self, config: &Config) -> Vec<PathBuf> {
        self.0
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
        let children: VecDeque<_> = self
            .0
            .iter()
            .map(|a| {
                a.stateful(io, res, config, sync_writer, async_writer)
                    .unwrap()
            })
            .collect();

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
    ) -> Result<()> {
        if let Some(c) = self.children.get_mut(0) {
            c.start(sync_writer, async_writer, state)
        } else {
            self.done = true;
            sync_writer.push(SyncSignal::UpdateGraph);
            Ok(())
        }
    }

    #[inline]
    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<()> {
        if let Some(c) = self.children.get_mut(0) {
            c.update(signal, sync_writer, async_writer, state)?;

            if c.is_over()? {
                self.children
                    .pop_front()
                    .unwrap()
                    .stop(sync_writer, async_writer, state)?;

                if let Some(c) = self.children.get_mut(0) {
                    c.start(sync_writer, async_writer, state)?;
                } else {
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
        if let Some(c) = self.children.get_mut(0) {
            c.show(ui, sync_writer, async_writer, state)
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
        if let Some(c) = self.children.get_mut(0) {
            c.stop(sync_writer, async_writer, state)?;
        }
        self.done = true;
        Ok(())
    }
}