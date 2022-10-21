use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE, VISUAL};
use crate::config::Config;
use crate::error;
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use eframe::egui;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Par(Vec<Box<dyn Action>>, #[serde(default)] Vec<Box<dyn Action>>);

stateful!(Par {
    primary: Vec<Box<dyn StatefulAction>>,
    secondary: Vec<Box<dyn StatefulAction>>,
});

impl Par {
    pub fn new(primary: Vec<Box<dyn Action>>, secondary: Vec<Box<dyn Action>>) -> Self {
        Self(primary, secondary)
    }
}

impl Action for Par {
    #[inline]
    fn resources(&self, config: &Config) -> Vec<PathBuf> {
        self.0
            .iter()
            .chain(self.1.iter())
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
        Ok(Box::new(StatefulPar {
            done: false,
            primary: self
                .0
                .iter()
                .map(|a| {
                    a.stateful(io, res, config, sync_writer, async_writer)
                        .unwrap()
                })
                .collect(),
            secondary: self
                .1
                .iter()
                .map(|a| {
                    a.stateful(io, res, config, sync_writer, async_writer)
                        .unwrap()
                })
                .collect(),
        }))
    }
}

impl StatefulPar {
    pub fn push_primary(&mut self, child: impl Into<Box<dyn StatefulAction>>) {
        self.primary.push(child.into());
    }

    pub fn push_secondary(&mut self, child: impl Into<Box<dyn StatefulAction>>) {
        self.secondary.push(child.into());
    }
}

impl StatefulAction for StatefulPar {
    impl_stateful!();

    #[inline]
    fn props(&self) -> Props {
        self.primary
            .iter()
            .chain(self.secondary.iter())
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
    ) -> Result<(), error::Error> {
        if self.primary.is_empty() {
            self.done = true;
            sync_writer.push(SyncSignal::UpdateGraph);
        } else {
            for c in self.primary.iter_mut().chain(self.secondary.iter_mut()) {
                c.start(sync_writer, async_writer)?;
            }
        }

        Ok(())
    }

    #[inline]
    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        let mut done = vec![];
        for (i, c) in self.primary.iter_mut().enumerate() {
            c.update(signal, sync_writer, async_writer)?;

            if c.is_over()? {
                c.stop(sync_writer, async_writer)?;
                done.push(i);
            }
        }
        for i in done.into_iter().rev() {
            self.primary.remove(i);
        }

        let mut done = vec![];
        for (i, c) in self.secondary.iter_mut().enumerate() {
            c.update(signal, sync_writer, async_writer)?;

            if c.is_over()? {
                c.stop(sync_writer, async_writer)?;
                done.push(i);
            }
        }
        for i in done.into_iter().rev() {
            self.secondary.remove(i);
        }

        if self.primary.is_empty() {
            self.done = true;
        }

        Ok(())
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        if let Some(c) = self
            .primary
            .iter_mut()
            .filter(|c| c.props().visual())
            .next()
        {
            c.show(ui, sync_writer, async_writer)
        } else if let Some(c) = self
            .secondary
            .iter_mut()
            .filter(|c| c.props().visual())
            .next()
        {
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
        let children = self.primary.iter_mut().chain(self.secondary.iter_mut());
        for c in children {
            c.stop(sync_writer, async_writer)?;
        }
        self.done = true;
        Ok(())
    }
}
