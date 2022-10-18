use crate::action::{Action, ActionEnum, StatefulAction, StatefulActionEnum};
use crate::config::Config;
use crate::error;
use crate::error::Error::{ActionEvolveError, InternalError, TaskDefinitionError};
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::resource::ResourceValue::Text;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Template {
    src: PathBuf,
    #[serde(default)]
    params: HashMap<String, String>,
}

impl Action for Template {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![self.src.to_owned()]
    }

    #[inline(always)]
    fn stateful(
        &self,
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<StatefulActionEnum, error::Error> {
        if let Text(inner) = res.fetch(&self.src)? {
            ron::from_str::<ActionEnum>(&inner)
                .map_err(|e| TaskDefinitionError(format!("{e:?}")))?
                .inner()
                .stateful(io, res, config, sync_writer, async_writer)
        } else {
            Err(TaskDefinitionError(format!(
                "`src` attribute of Template must be pointing to a `.ron` file: {:?}",
                self.src
            )))
        }
    }
}
