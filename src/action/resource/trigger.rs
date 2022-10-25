use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    Ext(PathBuf),
    Int,
    None,
}

impl Default for Trigger {
    #[inline(always)]
    fn default() -> Self {
        Trigger::None
    }
}
