use crate::action::image::Image;
use crate::action::{Action, StatefulAction};
use crate::comm::QWriter;
use crate::resource::{Color, ResourceMap};
use crate::server::{AsyncSignal, Config, SyncSignal, IO};
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fixation {
    #[serde(default)]
    width: Option<f32>,
    #[serde(default)]
    background: Color,
}

impl Action for Fixation {
    fn stateful(
        &self,
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Image::from(self).stateful(io, res, config, sync_writer, async_writer)
    }
}

impl From<&Fixation> for Image {
    fn from(fixation: &Fixation) -> Self {
        Self::new(
            PathBuf::from("fixation.svg"),
            fixation.width,
            fixation.background,
        )
    }
}
