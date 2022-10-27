use crate::action::Action;
use crate::comm::SignalId;
use crate::resource::ResourceAddr;
use crate::server::{config::OptionalConfig, Config, State};
use crate::util::Hash;
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::ser::to_vec_packed;
use serde_cbor::Value;
use std::collections::BTreeMap;

#[derive(Deserialize, Serialize, Debug)]
pub struct Block {
    name: String,
    #[serde(default)]
    #[serde(alias = "cfg")]
    config: OptionalConfig,
    tree: Box<dyn Action>,
    #[serde(default)]
    state: BTreeMap<SignalId, Value>,
}

impl Block {
    pub fn init(&mut self) -> Result<()> {
        self.verify_name()?;
        self.verify_connections()?;
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

    fn verify_connections(&self) -> Result<()> {
        let mut in_signals = self.tree.in_signals();
        let mut out_signals = self.tree.out_signals();

        in_signals.insert(0);
        out_signals.insert(0);

        let in_not_out: Vec<_> = in_signals.difference(&out_signals).collect();
        let out_not_in: Vec<_> = out_signals.difference(&in_signals).collect();

        if !in_not_out.is_empty() {
            Err(eyre!("Consumed signals are never produced: {in_not_out:?}"))
        } else if !out_not_in.is_empty() {
            Err(eyre!("Produced signals are never consumed: {out_not_in:?}"))
        } else {
            Ok(())
        }
    }

    pub fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
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
    pub fn default_state(&self) -> &State {
        &self.state
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
