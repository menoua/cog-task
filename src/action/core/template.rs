use crate::action::{Action, StatefulAction};
use crate::config::Config;
use crate::io::IO;
use crate::queue::QWriter;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::task::ROOT_DIR;
use eyre::{eyre, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Template {
    src: PathBuf,
    #[serde(default)]
    params: BTreeMap<String, String>,
}

impl Action for Template {
    fn init(self) -> Result<Box<dyn Action>> {
        let path = ROOT_DIR.get().unwrap().join(&self.src);
        let mut inner = fs::read_to_string(&path)
            .wrap_err_with(|| format!("Failed to read `Template` source: {path:?}"))?;

        for (k, v) in self.params.iter() {
            let re = regex::Regex::new(&format!(r"\$\{{{k}\}}")).unwrap();
            inner = re.replace_all(&inner, v).to_string();
        }

        ron::from_str::<Box<dyn Action>>(&inner).wrap_err("Failed to deserialize `Template`.")
    }

    #[inline]
    fn stateful(
        &self,
        _io: &IO,
        _res: &ResourceMap,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Err(eyre!("Template can not be stateful."))
    }
}
