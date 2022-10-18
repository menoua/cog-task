use crate::action::nil::Nil;
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
    #[serde(skip_deserializing)]
    #[serde(default = "defaults::inner")]
    inner: Box<ActionEnum>,
}

mod defaults {
    use super::*;

    pub fn inner() -> Box<ActionEnum> {
        Box::new(Nil.into())
    }
}

impl Action for Template {
    #[inline(always)]
    fn resources(&self, config: &Config) -> Vec<PathBuf> {
        self.inner.inner().resources(config)
    }

    fn init(&mut self, root_dir: &Path, config: &Config) -> Result<(), error::Error> {
        let path = root_dir.join(&self.src);
        let mut inner =
            fs::read_to_string(&path).map_err(|e| TaskDefinitionError(format!("{e:?}")))?;

        for (k, v) in self.params.iter() {
            let re = regex::Regex::new(&format!(r"\$\{{{k}\}}")).unwrap();
            inner = re.replace_all(&inner, v).to_string();
        }

        let inner = ron::from_str::<ActionEnum>(&inner)
            .map_err(|e| TaskDefinitionError(format!("{e:?}")))?;
        self.inner = Box::new(inner);

        self.inner.init(root_dir, config)
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
        self.inner
            .inner()
            .stateful(io, res, config, sync_writer, async_writer)
    }
}
