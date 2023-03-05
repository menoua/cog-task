use crate::action::{Action, ActionSignal, Props, StatefulAction};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{Color, IoManager, ResourceAddr, ResourceManager};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eframe::egui::{CentralPanel, Color32, Frame, Response, Ui};
use egui_extras::{Size, StripBuilder};
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Deserialize, Serialize)]
pub struct Rect([f32; 2], Box<dyn Action>, #[serde(default)] Color);

stateful!(Rect {
    size: [f32; 2],
    inner: Box<dyn StatefulAction>,
    background: Color32,
});

impl Action for Rect {
    fn in_signals(&self) -> BTreeSet<SignalId> {
        self.1.in_signals()
    }

    fn out_signals(&self) -> BTreeSet<SignalId> {
        self.1.out_signals()
    }

    fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
        self.1.resources(config)
    }

    fn stateful(
        &self,
        io: &IoManager,
        res: &ResourceManager,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Ok(Box::new(StatefulRect {
            done: false,
            size: self.0,
            inner: self
                .1
                .stateful(io, res, config, sync_writer, async_writer)?,
            background: self.2.into(),
        }))
    }
}

impl StatefulAction for StatefulRect {
    impl_stateful!();

    fn props(&self) -> Props {
        self.inner.props()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        self.inner.start(sync_writer, async_writer, state)
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        let news = self
            .inner
            .update(signal, sync_writer, async_writer, state)?;
        if self.inner.is_over()? {
            self.done = true;
        }
        Ok(news)
    }

    fn show(
        &mut self,
        ui: &mut Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Response> {
        let response = CentralPanel::default()
            .frame(Frame::default().fill(self.background))
            .show_inside(ui, |ui| {
                StripBuilder::new(ui)
                    .size(Size::remainder())
                    .size(Size::exact(self.size[0]))
                    .size(Size::remainder())
                    .horizontal(|mut strip| {
                        strip.empty();
                        strip.strip(|builder| {
                            builder
                                .size(Size::remainder())
                                .size(Size::exact(self.size[1]))
                                .size(Size::remainder())
                                .vertical(|mut strip| {
                                    strip.empty();
                                    strip.cell(|ui| {
                                        if let Err(e) =
                                            self.inner.show(ui, sync_writer, async_writer, state)
                                        {
                                            sync_writer.push(SyncSignal::Error(e));
                                        }
                                    });
                                    strip.empty();
                                });
                        });
                        strip.empty();
                    })
            })
            .inner;

        Ok(response)
    }

    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        self.inner.stop(sync_writer, async_writer, state)
    }
}
