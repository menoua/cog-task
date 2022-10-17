use crate::action::{Action, DEFAULT, FINITE, Props, StatefulAction, ActionEnum, StatefulActionEnum};
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
    ) -> Result<StatefulActionEnum, error::Error> {
        Ok(StatefulNop {
            id,
            done: false,
            persistent: self.persistent,
        }.into())
    }
}

impl StatefulAction for StatefulNop {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        if self.persistent {
            DEFAULT
        } else {
            FINITE
        }.into()
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        self.done = true;
        Ok(())
    }
}
