use crate::action::{Action, StatefulAction};
use crate::callback::CallbackQueue;
use crate::config::Config;
use crate::error;
use crate::error::Error::{InternalError, InvalidResourceError};
use crate::io::IO;
use crate::resource::{ResourceMap, ResourceValue};
use crate::scheduler::{AsyncCallback, SyncCallback};
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
        id: usize,
        res: &ResourceMap,
        _config: &Config,
        _io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        match res.fetch(&self.src)? {
            ResourceValue::Image(texture, size) => Ok(Box::new(StatefulImage {
                id,
                done: false,
                handle: texture,
                size,
                width: self.width,
            })),
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
    fn is_visual(&self) -> bool {
        true
    }

    #[inline(always)]
    fn is_static(&self) -> bool {
        true
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        self.done = true;
        Ok(())
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_queue: &mut CallbackQueue<SyncCallback>,
        async_queue: &mut CallbackQueue<AsyncCallback>,
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
