use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE, VISUAL};
use crate::comm::{QWriter, Signal};
use crate::resource::{ResourceAddr, ResourceMap};
use crate::server::{AsyncSignal, Config, State, SyncSignal, IO};
use eframe::egui;
use eyre::Result;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Par(
    Vec<Box<dyn Action>>,
    #[serde(default)] Vec<Box<dyn Action>>,
    #[serde(default)] Require,
);

stateful!(Par {
    primary: Vec<Box<dyn StatefulAction>>,
    secondary: Vec<Box<dyn StatefulAction>>,
    require: Require,
});

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Require {
    All,
    Any,
}

impl Default for Require {
    fn default() -> Self {
        Require::All
    }
}

impl Par {
    pub fn new(
        primary: Vec<Box<dyn Action>>,
        secondary: Vec<Box<dyn Action>>,
        require: Require,
    ) -> Self {
        Self(primary, secondary, require)
    }
}

impl Action for Par {
    #[inline]
    fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
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
    ) -> Result<Box<dyn StatefulAction>> {
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
            require: self.2,
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
        let mut props = DEFAULT;

        for c in self.primary.iter() {
            let c = c.props();
            if c.visual() {
                props |= VISUAL;
            }
            if c.infinite() {
                props |= INFINITE;
            }
        }

        for c in self.secondary.iter() {
            let c = c.props();
            if c.visual() {
                props |= VISUAL;
            }
        }

        props.into()
    }

    #[inline]
    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        let mut news = vec![];
        if self.primary.is_empty() {
            self.done = true;
            sync_writer.push(SyncSignal::UpdateGraph);
        } else {
            for c in self.primary.iter_mut().chain(self.secondary.iter_mut()) {
                news.extend(c.start(sync_writer, async_writer, state)?);
            }
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
        let mut done = vec![];
        let mut news = vec![];
        let mut finished = false;
        for (i, c) in self.primary.iter_mut().enumerate() {
            news.extend(c.update(signal, sync_writer, async_writer, state)?);

            if c.is_over()? {
                news.extend(c.stop(sync_writer, async_writer, state)?);
                done.push(i);
            }
        }
        if matches!(self.require, Require::Any) && !done.is_empty() {
            finished = true;
        }
        for i in done.into_iter().rev() {
            self.primary.remove(i);
        }
        if self.primary.is_empty() {
            finished = true;
        }

        let mut done = vec![];
        for (i, c) in self.secondary.iter_mut().enumerate() {
            news.extend(c.update(signal, sync_writer, async_writer, state)?);

            if c.is_over()? {
                news.extend(c.stop(sync_writer, async_writer, state)?);
                done.push(i);
            }
        }
        for i in done.into_iter().rev() {
            self.secondary.remove(i);
        }

        if finished {
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
        if let Some(c) = self.primary.iter_mut().find(|c| c.props().visual()) {
            c.show(ui, sync_writer, async_writer, state)
        } else if let Some(c) = self.secondary.iter_mut().find(|c| c.props().visual()) {
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
    ) -> Result<Signal> {
        self.done = true;
        let mut news = vec![];
        let children = self.primary.iter_mut().chain(self.secondary.iter_mut());
        for c in children {
            news.extend(c.stop(sync_writer, async_writer, state)?);
        }
        Ok(news.into())
    }
}
