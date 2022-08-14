use crate::config::Config;
use crate::error;
use crate::error::Error::{InvalidNameError, TaskDefinitionError};
use crate::util::Hash;
use block::Block;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub mod block;

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
        let path_json = root_dir.join("task.json");
        let path_yaml = root_dir.join("task.yml");
        let path = match (path_json.exists(), path_yaml.exists()) {
            (true, false) => path_json,
            (false, true) => path_yaml,
            _ => Err(TaskDefinitionError(
                "There should be exactly one of `task.json` or `task.yml` files in task directory."
                    .to_owned(),
            ))?,
        };

        let file = File::open(&path).map_err(|e| {
            TaskDefinitionError(format!(
                "Unable to open task definition file ({:?}):\n{e:#?}",
                path
            ))
        })?;

        match path.extension().unwrap().to_str().unwrap() {
            "json" => serde_json::from_reader::<_, Task>(BufReader::new(file)).map_err(|e| {
                TaskDefinitionError(format!("Invalid task definition file ({path:?}):\n{e:#?}"))
            })?,
            "yaml" => serde_yaml::from_reader::<_, Task>(BufReader::new(file)).map_err(|e| {
                TaskDefinitionError(format!("Invalid task definition file ({path:?}):\n{e:#?}"))
            })?,
            ext => Err(TaskDefinitionError(format!(
                "Invalid extension `{ext}` for task definition file ({path:?})"
            )))?,
        }
        .init(root_dir)
    }

    pub fn init(mut self, root_dir: &Path) -> Result<Self, error::Error> {
        for block in self.blocks.iter_mut() {
            block.init(root_dir, &self.config)?;
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
            let description = std::fs::read_to_string(&path).map_err(|e| {
                TaskDefinitionError(format!(
                    "Unable to open task description file ({path:?}):\n{e:#?}"
                ))
            })?;
            self.description = description;
        }

        self.config.verify()?;
        self.config.verify_checksum(self.hash())?;

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

    #[inline(always)]
    pub fn set_trigger(&mut self, state: bool) {
        self.config.set_trigger(state);
    }
}

impl Hash for Task {}
