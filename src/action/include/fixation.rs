use crate::action::{Action, StatefulAction, ActionEnum, StatefulActionEnum};
use crate::action::Image;
use crate::config::Config;
use crate::error;
use crate::error::Error::InternalError;
use crate::io::IO;
use crate::resource::ResourceMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

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
    ) -> Result<StatefulActionEnum, error::Error> {
        Err(InternalError(Self::LARVA_PANIC_MSG.to_owned()))
    }

    fn evolve(
        &self,
        _root_dir: &Path,
        _config: &Config,
    ) -> Result<Option<ActionEnum>, error::Error> {
        Ok(Some(Image::from(self).into()))
    }
}

impl Fixation {
    const LARVA_PANIC_MSG: &'static str =
        "Fixation is a larva action, it should be evolved into an \
        Image quickly after initialization";
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
