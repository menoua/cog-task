use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT};
use crate::config::Config;
use crate::error;
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use eframe::egui;
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
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        Ok(Box::new(StatefulSeq {
            done: false,
            children: self
                .0
                .iter()
                .map(|a| {
                    a.stateful(io, res, config, sync_writer, async_writer)
                        .unwrap()
                })
                .collect(),
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

    #[inline]
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
    ) -> Result<(), error::Error> {
        if let Some(c) = self.children.get_mut(0) {
            c.start(sync_writer, async_writer)
        } else {
            self.done = true;
            Ok(())
        }
    }

    #[inline]
    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        if let Some(c) = self.children.get_mut(0) {
            c.update(signal, sync_writer, async_writer)?;
        }

        if matches!(signal, ActionSignal::UpdateGraph) {
            while let Some(true) = self.children.get(0).map(|c| c.is_over().unwrap()) {
                self.children
                    .pop_front()
                    .unwrap()
                    .stop(sync_writer, async_writer)?;

                if let Some(c) = self.children.get_mut(0) {
                    c.start(sync_writer, async_writer)?;
                } else {
                    self.done = true;
                    sync_writer.push(SyncSignal::UpdateGraph);
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
    ) -> Result<(), error::Error> {
        if let Some(c) = self.children.get_mut(0) {
            c.show(ui, sync_writer, async_writer)
        } else {
            Ok(())
        }
    }

    #[inline]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        if let Some(c) = self.children.get_mut(0) {
            c.stop(sync_writer, async_writer)?;
        }
        self.done = true;
        Ok(())
    }
}
