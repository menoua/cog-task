pub mod block;

use crate::config::Config;
use crate::util::Hash;
use block::Block;
use eyre::{eyre, Context, Result};
use itertools::Itertools;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub static ROOT_DIR: OnceCell<PathBuf> = OnceCell::new();
pub static BASE_CFG: OnceCell<Config> = OnceCell::new();

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
    pub fn new(root_dir: &Path) -> Result<Self> {
        ROOT_DIR.set(root_dir.to_owned()).unwrap();

        let path = root_dir.join("task.ron");
        let content = fs::read_to_string(path).wrap_err("Failed to read task description file.")?;

        ron::from_str::<Task>(&content)
            .wrap_err("Failed to deserialize task description file.")?
            .init(root_dir)
    }

    pub fn init(mut self, root_dir: &Path) -> Result<Self> {
        for block in self.blocks.iter_mut() {
            block.init()?;
        }

        for (name, count) in self.block_labels().into_iter().counts() {
            if count > 1 {
                Err(eyre!(
                    "Block names have to be unique within a task ('{name}' is repeated)."
                ))?;
            }
        }

        if self.description.is_empty() {
            let path = root_dir.join("description.txt");
            let description = fs::read_to_string(&path)
                .wrap_err_with(|| format!("Unable to open task description file ({path:?})."))?;
            self.description = description;
        }

        // self.config.verify()?;
        self.config.verify_checksum(self.hash())?;
        BASE_CFG.set(self.config.clone()).unwrap();

        Ok(self)
    }

    #[inline(always)]
    pub fn name(&self) -> &String {
        &self.name
    }

    #[inline(always)]
    pub fn version(&self) -> &String {
        &self.version
    }

    #[inline(always)]
    pub fn title(&self) -> String {
        format!("{} ({})", self.name, self.version)
    }

    #[inline(always)]
    pub fn config(&self) -> &Config {
        &self.config
    }

    #[inline(always)]
    pub fn block(&self, i: usize) -> &Block {
        &self.blocks[i]
    }

    pub fn block_labels(&self) -> Vec<String> {
        self.blocks.iter().map(|b| b.label().to_string()).collect()
    }

    #[inline(always)]
    pub fn description(&self) -> &str {
        &self.description
    }
}

impl Hash for Task {}
