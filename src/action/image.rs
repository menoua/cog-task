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

#[derive(Debug)]
pub struct StatefulImage {
    id: usize,
    done: bool,
    // handle: Arc<image::Handle>,
    handle: TextureId,
    size: Vec2,
    width: Option<f32>,
}

impl StatefulAction for StatefulImage {
    #[inline(always)]
    fn id(&self) -> usize {
        self.id
    }

    #[inline(always)]
    fn is_over(&self) -> Result<bool, error::Error> {
        Ok(self.done)
    }

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
        ctx: &egui::Context,
        sync_queue: &mut CallbackQueue<SyncCallback>,
        async_queue: &mut CallbackQueue<AsyncCallback>,
    ) -> Result<(), error::Error> {
        CentralPanel::default().show(ctx, |ui| {
            ui.output().cursor_icon = CursorIcon::None;

            ui.centered_and_justified(|ui| {
                if let Some(width) = self.width {
                    let scale = width / self.size.x;
                    ui.image(self.handle, self.size * scale);
                } else {
                    ui.image(self.handle, self.size);
                }
            })
        });
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fixation {
    #[serde(default)]
    width: Option<f32>,
    #[serde(default)]
    style: String,
}

impl Action for Fixation {
    fn stateful(
        &self,
        _id: usize,
        _res: &ResourceMap,
        _config: &Config,
        _io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        Err(InternalError(Self::LARVA_PANIC_MSG.to_owned()))
    }

    fn evolve(
        &self,
        _root_dir: &Path,
        _config: &Config,
    ) -> Result<Option<Box<dyn Action>>, error::Error> {
        Ok(Some(Box::new(Image::from(self))))
    }
}

impl Fixation {
    const LARVA_PANIC_MSG: &'static str =
        "Fixation is a larva action, it should be evolved into an \
        Image quickly after initialization";
}

impl From<&Fixation> for Image {
    fn from(f: &Fixation) -> Self {
        Self {
            src: PathBuf::from("fixation.svg"),
            width: f.width,
            style: f.style.clone(),
        }
    }
}

impl From<Fixation> for Image {
    fn from(fixation: Fixation) -> Self {
        Self {
            src: PathBuf::from("fixation.svg"),
            width: fixation.width,
            style: fixation.style,
        }
    }
}
