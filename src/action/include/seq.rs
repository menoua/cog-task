use crate::action::{
    Action, ActionEnum, ActionSignal, Props, StatefulAction, StatefulActionEnum, CAP_KEYS, DEFAULT,
    VISUAL,
};
use crate::config::Config;
use crate::error;
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use eframe::egui;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
pub struct Seq(Vec<ActionEnum>);

stateful!(Seq {
    children: Vec<StatefulActionEnum>,
});

impl Seq {
    pub fn new(children: Vec<ActionEnum>) -> Self {
        Self(children)
    }
}

impl Action for Seq {
    #[inline]
    fn resources(&self, config: &Config) -> Vec<PathBuf> {
        self.0
            .iter()
            .flat_map(|c| c.inner().resources(config))
            .unique()
            .collect()
    }

    fn init(&mut self, root_dir: &Path, config: &Config) -> Result<(), error::Error> {
        for c in self.0.iter_mut() {
            c.inner_mut().init(root_dir, config)?;
        }
        Ok(())
    }

    fn stateful(
        &self,
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<StatefulActionEnum, error::Error> {
        Ok(StatefulSeq {
            id: 0,
            done: false,
            children: self
                .0
                .iter()
                .map(|a| {
                    a.inner()
                        .stateful(io, res, config, sync_writer, async_writer)
                        .unwrap()
                })
                .collect(),
        }
        .into())
    }
}

impl StatefulSeq {
    pub fn push(&mut self, child: impl Into<StatefulActionEnum>) {
        self.children.push(child.into());
    }
}

impl StatefulAction for StatefulSeq {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        if let Some(c) = self.children.get(0) {
            c.inner().props()
        } else {
            DEFAULT.into()
        }
    }

    #[inline(always)]
    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        if let Some(c) = self.children.get_mut(0) {
            c.inner_mut().start(sync_writer, async_writer)
        } else {
            self.done = true;
            Ok(())
        }
    }

    #[inline(always)]
    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        if let Some(c) = self.children.get_mut(0) {
            c.inner_mut().update(signal, sync_writer, async_writer)?;
        }

        if matches!(signal, ActionSignal::UpdateGraph) {
            if let Some(true) = self.children.get(0).map(|c| c.inner().is_over().unwrap()) {
                self.children.remove(0);

                if let Some(c) = self.children.get_mut(0) {
                    c.inner_mut().start(sync_writer, async_writer)?;
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
            c.inner_mut().show(ui, sync_writer, async_writer)
        } else {
            Ok(())
        }
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        for c in self.children.iter_mut() {
            c.inner_mut().stop()?;
        }
        self.done = true;
        Ok(())
    }
}
