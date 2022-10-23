use crate::action::Action;
use crate::config::{Config, OptionalConfig};
use crate::error;
use crate::error::Error::TaskDefinitionError;
use crate::util::Hash;
use serde::{Deserialize, Serialize};
use serde_cbor::ser::to_vec_packed;
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Debug)]
pub struct Block {
    name: String,
    tree: Box<dyn Action>,
    #[serde(default)]
    config: OptionalConfig,
}

impl Block {
    pub fn init(&mut self) -> Result<(), error::Error> {
        self.verify_name()?;
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
        self.tree.resources(config)
    }

    #[inline(always)]
    pub fn action_tree(&self) -> &dyn Action {
        &*self.tree
    }

    #[inline(always)]
    pub fn action_tree_vec(&self) -> Vec<u8> {
        to_vec_packed(&self.tree).unwrap()
    }

    #[inline(always)]
    pub fn label(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn config(&self, base_config: &Config) -> Config {
        self.config.fill_blanks(base_config)
    }
}

impl Hash for Block {}
