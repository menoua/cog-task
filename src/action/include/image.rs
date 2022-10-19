use crate::action::{
    Action, ActionEnum, ActionSignal, Props, StatefulAction, StatefulActionEnum, INFINITE, VISUAL,
};
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::error::Error::{InternalError, InvalidResourceError};
use crate::io::IO;
use crate::resource::{ResourceMap, ResourceValue};
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use eframe::egui;
use eframe::egui::{CentralPanel, CursorIcon, TextureId, Vec2};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Image {
    src: PathBuf,
    #[serde(default)]
    width: Option<f32>,
    #[serde(default)]
    style: String,
}

stateful!(Image {
    handle: TextureId,
    size: Vec2,
    width: Option<f32>,
});

impl Image {
    pub fn new(src: PathBuf, width: Option<f32>, style: String) -> Self {
        Self { src, width, style }
    }
}

impl Action for Image {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![self.src.to_owned()]
    }

    fn stateful(
        &self,
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<StatefulActionEnum, error::Error> {
        match res.fetch(&self.src)? {
            ResourceValue::Image(texture, size) => Ok(StatefulImage {
                id: 0,
                done: false,
                handle: texture,
                size,
                width: self.width,
            }
            .into()),
            _ => Err(InvalidResourceError(format!(
                "Image action supplied non-image resource: `{:?}`",
                self.src
            ))),
        }
    }
}

impl StatefulAction for StatefulImage {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        (INFINITE | VISUAL).into()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
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

    #[inline(always)]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
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
