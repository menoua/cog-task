use crate::action::{Action, ActionEnum};
use crate::config::{Config, OptionalConfig};
use crate::error;
use crate::error::Error::TaskDefinitionError;
use crate::util::Hash;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::slice::Iter;

#[derive(Deserialize, Serialize, Debug)]
pub struct Block {
    name: String,
    tree: ActionEnum,
    #[serde(default)]
    config: OptionalConfig,
}

impl Block {
    pub fn init(&mut self, root_dir: &Path, config: &Config) -> Result<(), error::Error> {
        let config = self.config(config);
        self.verify_name()?;
        self.tree.init(root_dir, &config)?;
        Ok(())
    }

    fn verify_name(&self) -> Result<(), error::Error> {
        if self.name.is_empty() {
            Err(TaskDefinitionError(
                "Block `name` cannot be the empty string".to_owned(),
            ))
        } else if !self
            .name
            .chars()
            .all(|c| c.is_alphabetic() || c.is_alphanumeric() | "+-_ ".contains(c))
        {
            Err(TaskDefinitionError(format!(
                "Block `name` characters need to be alphanumeric or one of (+-_ ): '{}'",
                self.name
            )))
        } else {
            Ok(())
        }
    }

    pub fn resources(&self, config: &Config) -> Vec<PathBuf> {
        self.tree.inner().resources(config)
    }

    #[inline(always)]
    pub fn action_tree(&self) -> &ActionEnum {
        &self.tree
    }

    #[inline(always)]
    pub fn label(&self) -> &str {
        &self.name
    }

    pub fn config(&self, base_config: &Config) -> Config {
        self.config.fill_blanks(base_config)
    }
}

impl Hash for Block {}
