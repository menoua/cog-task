use crate::action::{Action, ExtAction, StatefulAction};
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
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        Err(InternalError(Self::LARVA_PANIC_MSG.to_owned()))
    }

    fn evolve(
        &self,
        root_dir: &Path,
        config: &Config,
    ) -> Result<Option<Box<dyn Action>>, error::Error> {
        let path = root_dir.join(&self.src);
        if path.extension().is_none() {
            return Err(ActionEvolveError(format!(
                "Simple action's `src` argument ({:?}) should have an extension",
                self.src
            )));
        }

        let file = File::open(&path).map_err(|e| {
            ActionEvolveError(format!(
                "Unable to open simple action definition file ({path:?}):\n{e:#?}"
            ))
        })?;

        let action = match path.extension().map(|s| s.to_str().unwrap()) {
            Some("json") => {
                serde_json::from_reader::<_, ExtAction>(BufReader::new(file)).map_err(|e| {
                    TaskDefinitionError(format!("Invalid simple action file ({path:?}):\n{e:#?}"))
                })
            }
            Some("yaml") => {
                serde_yaml::from_reader::<_, ExtAction>(BufReader::new(file)).map_err(|e| {
                    TaskDefinitionError(format!("Invalid simple action file ({path:?}):\n{e:#?}"))
                })
            }
            _ => Err(TaskDefinitionError(format!(
                "Simple action source ({path:?}) should be a `.json`, `.yml`, or `.ron` file"
            ))),
        }?
        .action;

        Ok(if config.nested_evolve() {
            if let Some(nested_action) = action.evolve(root_dir, config)? {
                Some(nested_action)
            } else {
                Some(action)
            }
        } else {
            Some(action)
        })
    }
}

impl Simple {
    const LARVA_PANIC_MSG: &'static str = "Simple is a larva action, it should be evolved into a \
        fully functional action quickly after initialization";
}
