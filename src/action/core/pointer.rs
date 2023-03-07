use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{
    IoManager, Mask2D, OptionalFloat, OptionalPath, ResourceAddr, ResourceManager, ResourceValue,
};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eframe::egui::{CursorIcon, Response, Sense, Ui};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::BTreeSet;
use std::time::Instant;

#[derive(Debug, Deserialize, Serialize)]
pub struct Pointer {
    inner: Box<dyn Action>,
    #[serde(default)]
    group: String,
    #[serde(default)]
    until: Until,
    #[serde(default)]
    mask: OptionalPath,
    #[serde(default)]
    mask_width: OptionalFloat,
    #[serde(default)]
    out_rt: SignalId,
    #[serde(default)]
    out_coord: SignalId,
    #[serde(default)]
    out_accuracy: SignalId,
    #[serde(default)]
    out_hit: SignalId,
}

stateful!(Pointer {
    inner: Box<dyn StatefulAction>,
    // size: Vec2,
    _group: String,
    until: Until,
    mask: Option<Mask2D>,
    out_rt: SignalId,
    out_coord: SignalId,
    out_accuracy: SignalId,
    out_hit: SignalId,
    since: Instant,
});

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Until {
    None,
    Hit,
    Hits(u32),
    Click,
    Clicks(u32),
}

impl Default for Until {
    fn default() -> Self {
        Self::None
    }
}

impl Action for Pointer {
    fn init(self) -> Result<Box<dyn Action>>
    where
        Self: 'static + Sized,
    {
        if self.out_rt == 0
            && self.out_coord == 0
            && self.out_accuracy == 0
            && self.out_hit == 0
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
        signals.insert(self.out_hit);
        signals
    }

    fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
        let mut resources = self.inner.resources(config);
        if let Some(path) = self.mask.as_ref() {
            resources.push(ResourceAddr::Mask(path.to_owned()));
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
        let mask = match self.mask.as_ref() {
            Some(mask) => {
                let mask = ResourceAddr::Mask(mask.to_owned());
                if let ResourceValue::Mask(mask) = res.fetch(&mask)? {
                    if let Some(width) = self.mask_width.as_f32() {
                        Some(mask.scaled(mask.size().x / width))
                    } else {
                        Some(mask)
                    }
                } else {
                    return Err(eyre!("Resource value and address types don't match."));
                }
            }
            None => None,
        };

        Ok(Box::new(StatefulPointer {
            done: false,
            inner: self
                .inner
                .stateful(io, res, config, sync_writer, async_writer)?,
            // size: Vec2::from(self.size),
            _group: self.group.clone(),
            until: self.until,
            mask,
            out_rt: self.out_rt,
            out_coord: self.out_coord,
            out_accuracy: self.out_accuracy,
            out_hit: self.out_hit,
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
            let coord = response.interact_pointer_pos().ok_or_else(|| {
                eyre!("Pointer clicked but interact position could not be obtained.")
            })?;
            let coord = coord - corner;

            let score = if let Some(mask) = &self.mask {
                mask.value_at(coord)
            } else {
                1.0
            };

            #[cfg(debug_assertions)]
            println!("Clicked {coord:?} -> {score}");

            let mut signals = vec![];
            if self.out_rt > 0 {
                let rt = (time - self.since).as_secs_f32();
                signals.push((self.out_rt, Value::Float(rt as f64)));
            }
            if self.out_coord > 0 {
                signals.push((
                    self.out_coord,
                    Value::Array(vec![
                        Value::Float(coord.x as f64),
                        Value::Float(coord.y as f64),
                    ]),
                ));
            }
            if self.out_accuracy > 0 {
                signals.push((self.out_accuracy, Value::Float(score as f64)));
            }
            if self.out_hit > 0 {
                signals.push((self.out_hit, Value::Bool(score > 0.0)));
            }

            sync_writer.push(SyncSignal::Emit(time, signals.into()));

            let mut is_done = false;
            self.until = match (self.until, score) {
                (Until::Hit, s) if s > 0.0 => {
                    is_done = true;
                    Until::Hit
                }
                (Until::Hits(n), s) if s > 0.0 => {
                    if n == 1 {
                        is_done = true;
                    }
                    Until::Hits(n.saturating_sub(1))
                }
                (Until::Click, _) => {
                    is_done = true;
                    Until::Click
                }
                (Until::Clicks(n), _) => {
                    if n == 1 {
                        is_done = true;
                    }
                    Until::Clicks(n.saturating_sub(1))
                }
                (until, _) => until,
            };

            if is_done {
                self.done = true;
                sync_writer.push(SyncSignal::UpdateGraph);
            }
        }

        ui.output_mut(|o| o.cursor_icon = CursorIcon::Default);

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
