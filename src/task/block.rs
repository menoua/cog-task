use crate::action::Action;
use crate::config::{Config, OptionalConfig};
use crate::util::Hash;
use eyre::{eyre, Result};
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
    pub fn init(&mut self) -> Result<()> {
        self.verify_name()?;
        Ok(())
    }

    fn verify_name(&self) -> Result<()> {
        if self.name.is_empty() {
            Err(eyre!("Block `name` cannot be the empty string."))
        } else if !self
            .name
            .chars()
            .all(|c| c.is_alphabetic() || c.is_alphanumeric() | "+-_ ".contains(c))
        {
            Err(eyre!(
                "Block `name` characters need to be alphanumeric or one of (+-_ ): '{}'",
                self.name
            ))
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

impl Hash for Block {
    fn hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::default();
        hasher.update(&serde_cbor::to_vec(&(&self.tree, &self.config)).unwrap());
        hex::encode(hasher.finalize())
    }
}
