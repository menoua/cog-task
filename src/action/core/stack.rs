use crate::action::{
    Action, ActionSignal, Props, StatefulAction, StatefulNil, DEFAULT, INFINITE, VISUAL,
};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{IoManager, ResourceAddr, ResourceManager};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eframe::egui;
use eframe::egui::Response;
use egui_extras::{Size, StripBuilder};
use eyre::{eyre, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Deserialize, Serialize)]
pub struct Stack(
    Vec<Box<dyn Action>>,
    #[serde(default)] Direction,
    #[serde(default)] Vec<f32>,
);

stateful!(Stack {
    children: Vec<Box<dyn StatefulAction>>,
    direction: Direction,
    active: Vec<bool>,
    proportions: Vec<f32>,
});

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Direction {
    Horizontal,
    Vertical,
}

impl Default for Direction {
    fn default() -> Self {
        Self::Horizontal
    }
}

impl Stack {
    pub fn new(children: Vec<Box<dyn Action>>, dir: Direction, proportions: Vec<f32>) -> Self {
        Self(children, dir, proportions)
    }
}

impl Action for Stack {
    fn init(self) -> Result<Box<dyn Action>>
    where
        Self: 'static + Sized,
    {
        if !self.2.is_empty() && self.2.len() != self.0.len() {
            return Err(eyre!(
                "Stack should have same number of proportions and children."
            ));
        }

        if self.2.iter().sum::<f32>() > 1.0 {
            return Err(eyre!("Sum of Stack proportions cannot be greater than 1."));
        }

        Ok(Box::new(self))
    }

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
        let mut children = vec![];
        for c in self.0.iter() {
            children.push(c.stateful(io, res, config, sync_writer, async_writer)?);
        }

        let active = (0..children.len()).map(|_| true).collect();

        let proportions = if self.2.is_empty() {
            let count = self.0.len();
            (0..count).map(|_| 1.0 / count as f32).collect()
        } else {
            self.2.clone()
        };

        Ok(Box::new(StatefulStack {
            done: false,
            children,
            direction: self.1,
            active,
            proportions,
        }))
    }
}

impl StatefulAction for StatefulStack {
    impl_stateful!();

    #[inline]
    fn props(&self) -> Props {
        let mut props = DEFAULT;

        for c in self.children.iter() {
            let c = c.props();
            if c.visual() {
                props |= VISUAL;
            }
            if c.infinite() {
                props |= INFINITE;
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
        if self.children.is_empty() {
            self.done = true;
            sync_writer.push(SyncSignal::UpdateGraph);
        } else {
            for c in self.children.iter_mut() {
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
        let mut news = vec![];
        for (i, c) in self.children.iter_mut().enumerate() {
            if !self.active[i] {
                continue;
            }

            news.extend(c.update(signal, sync_writer, async_writer, state)?);

            if c.is_over()? {
                *c = Box::<StatefulNil>::default();
                self.active[i] = false;
            }
        }
        if !self.active.iter().any(|&c| c) {
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
    ) -> Result<Response> {
        let mut builder = StripBuilder::new(ui).clip(true);
        builder = builder.size(Size::remainder());
        for &p in self.proportions.iter() {
            builder = builder.size(Size::relative(p));
        }
        builder = builder.size(Size::remainder());

        let response = match self.direction {
            Direction::Horizontal => builder.horizontal(|mut strip| {
                strip.empty();
                for c in self.children.iter_mut() {
                    strip.cell(|ui| {
                        if let Err(e) = c.show(ui, sync_writer, async_writer, state) {
                            sync_writer.push(SyncSignal::Error(e));
                        }
                    });
                }
                strip.empty();
            }),
            Direction::Vertical => builder.vertical(|mut strip| {
                strip.empty();
                for c in self.children.iter_mut() {
                    strip.cell(|ui| {
                        if let Err(e) = c.show(ui, sync_writer, async_writer, state) {
                            sync_writer.push(SyncSignal::Error(e));
                        }
                    });
                }
                strip.empty();
            }),
        };

        Ok(response)
    }

    #[inline]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        let mut news = vec![];
        for c in self.children.iter_mut() {
            news.extend(c.stop(sync_writer, async_writer, state)?);
        }
        Ok(news.into())
    }
}
