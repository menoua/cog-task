use crate::action::{
    Action, ActionEnum, ActionSignal, Props, StatefulAction, StatefulActionEnum, CAP_KEYS, DEFAULT,
    INFINITE, VISUAL,
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
pub struct Par(Vec<ActionEnum>, #[serde(default)] Vec<ActionEnum>);

stateful!(Par {
    children: Vec<StatefulActionEnum>,
    children_opt: Vec<StatefulActionEnum>,
});

impl Par {
    pub fn new(children: Vec<ActionEnum>, children_opt: Vec<ActionEnum>) -> Self {
        Self(children, children_opt)
    }
}

impl Action for Par {
    #[inline]
    fn resources(&self, config: &Config) -> Vec<PathBuf> {
        self.0
            .iter()
            .chain(self.1.iter())
            .flat_map(|c| c.inner().resources(config))
            .unique()
            .collect()
    }

    fn init(&mut self, root_dir: &Path, config: &Config) -> Result<(), error::Error> {
        let children = self.0.iter_mut().chain(self.1.iter_mut());
        for c in children {
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
        Ok(StatefulPar {
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
            children_opt: self
                .1
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

impl StatefulPar {
    pub fn push(&mut self, child: impl Into<StatefulActionEnum>) {
        self.children.push(child.into());
    }

    pub fn push_opt(&mut self, child: impl Into<StatefulActionEnum>) {
        self.children_opt.push(child.into());
    }
}

impl StatefulAction for StatefulPar {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        let children = self.children.iter().chain(self.children_opt.iter());
        children
            .fold(DEFAULT, |mut state, c| {
                let c = c.inner().props();
                if c.visual() {
                    state |= VISUAL;
                }
                if c.infinite() {
                    state |= INFINITE;
                }
                if c.captures_keys() {
                    state |= CAP_KEYS;
                }
                state
            })
            .into()
    }

    #[inline(always)]
    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        let children = self.children.iter_mut().chain(self.children_opt.iter_mut());
        for c in children {
            c.inner_mut().start(sync_writer, async_writer)?;
        }

        if self.children.is_empty() {
            self.done = true;
        }
        Ok(())
    }

    #[inline(always)]
    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        let children = self.children.iter_mut().chain(self.children_opt.iter_mut());
        for c in children {
            c.inner_mut().update(signal, sync_writer, async_writer)?;
        }

        if matches!(signal, ActionSignal::UpdateGraph) {
            self.children.retain(|c| !c.inner().is_over().unwrap());
            self.children_opt.retain(|c| !c.inner().is_over().unwrap());
            if self.children.is_empty() {
                self.done = true;
                sync_writer.push(SyncSignal::UpdateGraph);
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
        let children = self.children.iter_mut().chain(self.children_opt.iter_mut());
        if let Some(c) = children.filter(|c| c.inner().props().visual()).next() {
            c.inner_mut().show(ui, sync_writer, async_writer)
        } else {
            Ok(())
        }
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        let children = self.children.iter_mut().chain(self.children_opt.iter_mut());
        for c in children {
            c.inner_mut().stop()?;
        }
        self.done = true;
        Ok(())
    }
}
