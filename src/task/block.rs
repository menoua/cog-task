use crate::action::{Action, ExtAction};
use crate::config::{Config, OptionalConfig};
use crate::error;
use crate::error::Error::TaskDefinitionError;
use crate::scheduler::flow::Flow;
use crate::util::Hash;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::slice::Iter;

#[derive(Deserialize, Serialize, Debug)]
pub struct Block {
    name: String,
    actions: Vec<ExtAction>,
    #[serde(default)]
    flow: Flow,
    #[serde(default)]
    config: Option<OptionalConfig>,
}

impl Block {
    #[inline(always)]
    pub fn iter(&self) -> Iter<'_, ExtAction> {
        self.actions.iter()
    }

    pub fn init(&mut self, root_dir: &Path, config: &Config) -> Result<(), error::Error> {
        let config = self.config(config);
        self.verify_name()?;

        for action in self.actions.iter_mut() {
            action.init(root_dir, &config)?;
        }

        for (name, count) in self
            .actions
            .iter()
            .filter_map(|action| action.id())
            .into_iter()
            .counts()
        {
            if count > 1 {
                Err(TaskDefinitionError(format!(
                    "Block names have to be unique within a task: '{name}' is repeated"
                )))?;
            }
        }

        let id_map = self
            .actions
            .iter()
            .enumerate()
            .filter_map(|(i, action)| action.id().map(|label| (label, i)))
            .collect::<HashMap<&String, usize>>();
        self.flow.normalize(id_map)?;

        if self.flow.fallback(self.actions.len()) {
            println!("II: Using sequential flow for block \"{}\".", self.name);
        }

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
        self.actions
            .iter()
            .flat_map(|action| action.inner().resources(config))
            .collect()
    }

    #[inline(always)]
    pub fn actions(&self) -> &Vec<ExtAction> {
        &self.actions
    }

    #[inline(always)]
    pub fn flow(&self) -> &Flow {
        &self.flow
    }

    #[inline(always)]
    pub fn label(&self) -> &str {
        &self.name
    }

    #[inline(always)]
    pub fn action(&self, i: usize) -> &dyn Action {
        self.actions[i].inner()
    }

    pub fn config(&self, base_config: &Config) -> Config {
        if let Some(config) = &self.config {
            config.fill_blanks(base_config)
        } else {
            base_config.clone()
        }
    }
}

impl Hash for Block {}
