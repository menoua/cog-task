use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE, VISUAL};
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::error::Error::InvalidResourceError;
use crate::io::IO;
use crate::resource::{ResourceMap, ResourceValue};
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use eframe::egui;
use eframe::egui::{CursorIcon, TextureId, Vec2};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Image {
    src: PathBuf,
    #[serde(default)]
    width: Option<f32>,
}

stateful!(Image {
    handle: TextureId,
    size: Vec2,
    width: Option<f32>,
});

impl Image {
    pub fn new(src: PathBuf, width: Option<f32>) -> Self {
        Self { src, width }
    }
}

impl Action for Image {
    #[inline]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![self.src.to_owned()]
    }

    fn stateful(
        &self,
        _io: &IO,
        res: &ResourceMap,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        let (texture, size) = {
            if let ResourceValue::Image(texture, size) = res.fetch(&self.src)? {
                (texture, size)
            } else {
                return Err(InvalidResourceError(format!(
                    "Image action supplied non-image resource: `{:?}`",
                    self.src
                )));
            }
        };

        Ok(Box::new(StatefulImage {
            done: false,
            handle: texture,
            size,
            width: self.width,
        }))
    }
}

impl StatefulAction for StatefulImage {
    impl_stateful!();

    #[inline]
    fn props(&self) -> Props {
        (INFINITE | VISUAL).into()
    }

    fn start(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn update(
        &mut self,
        _signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        ui.output().cursor_icon = CursorIcon::None;

        ui.centered_and_justified(|ui| {
            if let Some(width) = self.width {
                let scale = width / self.size.x;
                ui.image(self.handle, self.size * scale);
            } else {
                ui.image(self.handle, self.size);
            }
        });

        Ok(())
    }

    #[inline]
    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        self.done = true;
        Ok(())
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([
                ("texture_id", format!("{:?}", self.handle)),
                ("size", format!("{:?}", self.size)),
            ])
            .collect()
    }
}
