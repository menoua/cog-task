use crate::config::Config;
use crate::error;
use crate::error::Error::{InvalidNameError, TaskDefinitionError};
use crate::util::Hash;
use block::Block;
use itertools::Itertools;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub mod block;

pub static ROOT_DIR: OnceCell<PathBuf> = OnceCell::new();
pub static CONFIG: OnceCell<Config> = OnceCell::new();

#[derive(Deserialize, Debug, Default, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    name: String,
    version: String,
    blocks: Vec<Block>,
    #[serde(default)]
    config: Config,
    #[serde(default)]
    description: String,
}

impl Task {
    pub fn new(root_dir: &Path) -> Result<Self, error::Error> {
        ROOT_DIR.set(root_dir.to_owned()).unwrap();

        let path = root_dir.join("task.ron");
        let content =
            fs::read_to_string(path).map_err(|e| TaskDefinitionError(format!("{e:?}")))?;

        ron::from_str::<Task>(&content)
            .map_err(|e| TaskDefinitionError(format!("{e:?}")))?
            .init(root_dir)
    }

    pub fn init(mut self, root_dir: &Path) -> Result<Self, error::Error> {
        for block in self.blocks.iter_mut() {
            block.init()?;
        }

        for (name, count) in self.block_labels().into_iter().counts() {
            if count > 1 {
                Err(InvalidNameError(format!(
                    "Block names have to be unique within a task: '{name}' is repeated"
                )))?;
            }
        }

        if self.description.is_empty() {
            let path = root_dir.join("description.text");
            let description = fs::read_to_string(&path).map_err(|e| {
                TaskDefinitionError(format!(
                    "Unable to open task description file ({path:?}):\n{e:#?}"
                ))
            })?;
            self.description = description;
        }

        // self.config.verify()?;
        self.config.verify_checksum(self.hash())?;
        CONFIG.set(self.config.clone()).unwrap();

        Ok(self)
    }

    #[inline]
    pub fn name(&self) -> &String {
        &self.name
    }

    #[inline]
    pub fn version(&self) -> &String {
        &self.version
    }

    #[inline]
    pub fn title(&self) -> String {
        format!("{} ({})", self.name, self.version)
    }

    #[inline]
    pub fn config(&self) -> &Config {
        &self.config
    }

    #[inline]
    pub fn block(&self, i: usize) -> &Block {
        &self.blocks[i]
    }

    pub fn block_labels(&self) -> Vec<String> {
        self.blocks.iter().map(|b| b.label().to_string()).collect()
    }

    #[inline]
    pub fn description(&self) -> &str {
        &self.description
    }
}

impl Hash for Task {}
