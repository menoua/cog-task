use crate::action::{Action, StatefulAction};
use crate::config::Config;
use crate::error;
use crate::error::Error::{InternalError, TaskDefinitionError};
use crate::io::IO;
use crate::queue::QWriter;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::task::ROOT_DIR;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Template {
    src: PathBuf,
    #[serde(default)]
    params: HashMap<String, String>,
}

impl Action for Template {
    fn init(self) -> Result<Box<dyn Action>, error::Error> {
        let path = ROOT_DIR.get().unwrap().join(&self.src);
        let mut inner =
            fs::read_to_string(&path).map_err(|e| TaskDefinitionError(format!("{e:?}")))?;

        for (k, v) in self.params.iter() {
            let re = regex::Regex::new(&format!(r"\$\{{{k}\}}")).unwrap();
            inner = re.replace_all(&inner, v).to_string();
        }

        ron::from_str::<Box<dyn Action>>(&inner).map_err(|e| TaskDefinitionError(format!("{e:?}")))
    }

    #[inline]
    fn stateful(
        &self,
        _io: &IO,
        _res: &ResourceMap,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        Err(InternalError("Template can not be stateful".to_owned()))
    }
}
