use crate::action::Image;
use crate::action::{Action, ActionEnum, StatefulAction, StatefulActionEnum};
use crate::config::Config;
use crate::error;
use crate::error::Error::InternalError;
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<StatefulActionEnum, error::Error> {
        Image::from(self)
            .stateful(io, res, config, sync_writer, async_writer)
    }
}

impl From<&Fixation> for Image {
    fn from(fixation: &Fixation) -> Self {
        Self::new(
            PathBuf::from("fixation.svg"),
            fixation.width,
            fixation.style.clone(),
        )
    }
}

impl From<Fixation> for Image {
    fn from(fixation: Fixation) -> Self {
        Self::new(
            PathBuf::from("fixation.svg"),
            fixation.width,
            fixation.style,
        )
    }
}
