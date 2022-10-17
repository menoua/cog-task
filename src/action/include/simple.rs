use std::fs;
use crate::action::{Action, ActionEnum, StatefulAction, StatefulActionEnum};
use crate::config::Config;
use crate::error;
use crate::error::Error::{ActionEvolveError, InternalError, TaskDefinitionError};
use crate::io::IO;
use crate::resource::ResourceMap;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Simple {
    src: PathBuf,
}

impl Action for Simple {
    #[inline(always)]
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
        root_dir: &Path,
        config: &Config,
    ) -> Result<Option<ActionEnum>, error::Error> {
        let path = root_dir.join(&self.src);
        let content =
            fs::read_to_string(path).map_err(|e| TaskDefinitionError(format!("{e:?}")))?;

        let action = ron::from_str::<ActionEnum>(&content)
            .map_err(|e| TaskDefinitionError(format!("{e:?}")))?;

        // Ok(if config.nested_evolve() {
        //     if let Some(nested_action) = action.evolve(root_dir, config)? {
        //         Some(nested_action)
        //     } else {
        //         Some(action)
        //     }
        // } else {
        //     Some(action)
        // })
        Ok(Some(action))
    }
}

impl Simple {
    const LARVA_PANIC_MSG: &'static str = "Simple is a larva action, it should be evolved into a \
        fully functional action quickly after initialization";
}
