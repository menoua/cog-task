use crate::action::{Action, StatefulAction};
use crate::config::Config;
use crate::error;
use crate::io::IO;
use crate::resource::ResourceMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Nop {
    #[serde(default = "defaults::persistent")]
    #[serde(rename = "static")]
    persistent: bool,
}

stateful!(Nop { persistent: bool });

mod defaults {
    #[inline(always)]
    pub fn persistent() -> bool {
        false
    }
}

impl Action for Nop {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![]
    }

    fn stateful(
        &self,
        id: usize,
        _res: &ResourceMap,
        _config: &Config,
        _io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        Ok(Box::new(StatefulNop {
            id,
            done: false,
            persistent: self.persistent,
        }))
    }
}

impl StatefulAction for StatefulNop {
    impl_stateful!();

    #[inline(always)]
    fn is_visual(&self) -> bool {
        false
    }

    #[inline(always)]
    fn is_static(&self) -> bool {
        self.persistent
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        self.done = true;
        Ok(())
    }
}
