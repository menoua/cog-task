use crate::action::{Action, Props, StatefulAction, INFINITE, VISUAL};
use crate::comm::QWriter;
use crate::resource::{
    Color, IoManager, OptionalFloat, ResourceAddr, ResourceManager, ResourceValue,
};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eframe::egui;
use eframe::egui::{CentralPanel, Color32, Frame, Response, TextureId, Vec2};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Image {
    src: PathBuf,
    #[serde(default)]
    width: OptionalFloat,
    #[serde(default)]
    background: Color,
    #[serde(default = "defaults::pad")]
    pad: bool,
}

stateful!(Image {
    handle: TextureId,
    size: Vec2,
    width: Option<f32>,
    background: Color32,
    pad: bool,
});

mod defaults {
    pub fn pad() -> bool {
        true
    }
}

impl Image {
    #[inline(always)]
    pub fn new(src: PathBuf, width: Option<f32>, background: Color, pad: bool) -> Self {
        Self {
            src,
            width: width.into(),
            background,
            pad,
        }
    }
}

impl Action for Image {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<ResourceAddr> {
        vec![ResourceAddr::Image(self.src.to_owned())]
    }

    fn stateful(
        &self,
        _io: &IoManager,
        res: &ResourceManager,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        let src = ResourceAddr::Image(self.src.clone());
        let (texture, size) = {
            if let ResourceValue::Image(texture, size) = res.fetch(&src)? {
                (texture, size)
            } else {
                return Err(eyre!("Resource value and address types don't match."));
            }
        };

        Ok(Box::new(StatefulImage {
            done: false,
            handle: texture,
            size,
            width: self.width.as_f32(),
            background: self.background.into(),
            pad: self.pad,
        }))
    }
}

impl StatefulAction for StatefulImage {
    impl_stateful!();

    #[inline]
    fn props(&self) -> Props {
        (INFINITE | VISUAL).into()
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Response> {
        let scale = if let Some(width) = self.width {
            width / self.size.x
        } else {
            1.0
        };

        let response = CentralPanel::default()
            .frame(Frame::default().fill(self.background))
            .show_inside(ui, |ui| {
                if self.pad {
                    ui.centered_and_justified(|ui| ui.image(self.handle, self.size * scale))
                        .inner
                } else {
                    ui.image(self.handle, self.size * scale)
                }
            })
            .inner;

        Ok(response)
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
