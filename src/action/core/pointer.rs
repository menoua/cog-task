use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{IoManager, OptionalPath, ResourceAddr, ResourceManager};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eframe::egui::{CursorIcon, Response, Sense, Ui, Vec2};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::BTreeSet;
use std::time::Instant;

#[derive(Debug, Deserialize, Serialize)]
pub struct Pointer {
    inner: Box<dyn Action>,
    // size: [f32; 2],
    #[serde(default)]
    group: String,
    #[serde(default)]
    mask: OptionalPath,
    #[serde(default)]
    out_rt: SignalId,
    #[serde(default)]
    out_coord: SignalId,
    #[serde(default)]
    out_accuracy: SignalId,
}

stateful!(Pointer {
    inner: Box<dyn StatefulAction>,
    // size: Vec2,
    group: String,
    mask: OptionalPath,
    out_rt: SignalId,
    out_coord: SignalId,
    out_accuracy: SignalId,
    since: Instant,
});

impl Action for Pointer {
    fn init(self) -> Result<Box<dyn Action>>
    where
        Self: 'static + Sized,
    {
        if self.out_rt == 0
            && self.out_coord == 0
            && self.out_accuracy == 0
            && self.group.is_empty()
        {
            return Err(eyre!(
                "Pointer with no `out_*` signal and no `group` is useless."
            ));
        }

        Ok(Box::new(self))
    }

    fn in_signals(&self) -> BTreeSet<SignalId> {
        self.inner.in_signals()
    }

    fn out_signals(&self) -> BTreeSet<SignalId> {
        let mut signals = self.inner.out_signals();
        signals.insert(self.out_rt);
        signals.insert(self.out_coord);
        signals.insert(self.out_accuracy);
        signals
    }

    fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
        let mut resources = self.inner.resources(config);
        if let OptionalPath::Some(path) = &self.mask {
            resources.push(ResourceAddr::Image(path.clone()));
        }
        resources
    }

    fn stateful(
        &self,
        io: &IoManager,
        res: &ResourceManager,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Ok(Box::new(StatefulPointer {
            done: false,
            inner: self
                .inner
                .stateful(io, res, config, sync_writer, async_writer)?,
            // size: Vec2::from(self.size),
            group: self.group.clone(),
            mask: self.mask.clone(),
            out_rt: self.out_rt,
            out_coord: self.out_coord,
            out_accuracy: self.out_accuracy,
            since: Instant::now(),
        }))
    }
}

impl StatefulAction for StatefulPointer {
    impl_stateful!();

    fn props(&self) -> Props {
        (self.inner.props().bits() & !INFINITE).into()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        self.since = Instant::now();
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
        let corner = ui.next_widget_position();
        let response = self
            .inner
            .show(ui, sync_writer, async_writer, state)?
            .interact(Sense::click());

        if response.clicked() {
            let time = Instant::now();
            let coord = if let Some(coord) = response.interact_pointer_pos() {
                coord - corner
            } else {
                return Ok(response);
            };

            println!("{coord:?}");

            if self.out_rt > 0 {
                let rt = (time - self.since).as_secs_f32();
                sync_writer.push(SyncSignal::Emit(
                    time,
                    vec![(self.out_rt, Value::Float(rt as f64))].into(),
                ))
            }
            if self.out_coord > 0 {
                sync_writer.push(SyncSignal::Emit(
                    time,
                    vec![(
                        self.out_coord,
                        Value::Array(vec![
                            Value::Float(coord.x as f64),
                            Value::Float(coord.y as f64),
                        ]),
                    )]
                    .into(),
                ))
            }
            if self.out_accuracy > 0 {
                sync_writer.push(SyncSignal::Emit(
                    time,
                    vec![(self.out_accuracy, Value::Float(0.0))].into(),
                ))
            }

            self.done = true;
            sync_writer.push(SyncSignal::UpdateGraph);
        }

        ui.ctx().output().cursor_icon = CursorIcon::Default;

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
