use crate::assets::VERSION;
use crate::server::{Block, Server, Task};
use crate::util::Hash;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Info {
    subject: String,
    output: PathBuf,
    server: ServerInfo,
    task: TaskInfo,
    block: BlockInfo,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct TaskInfo {
    name: String,
    version: String,
    hash: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct BlockInfo {
    name: String,
    hash: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ServerInfo {
    version: String,
    hash: String,
}

impl Info {
    pub fn new(server: &Server, task: &Task, block: &Block) -> Self {
        Self {
            subject: server.subject().to_owned(),
            output: server.env().output().join(server.subject()),
            server: ServerInfo {
                version: VERSION.to_owned(),
                hash: server.hash(),
            },
            task: TaskInfo {
                name: task.name().to_owned(),
                version: task.version().to_owned(),
                hash: task.hash(),
            },
            block: BlockInfo {
                name: block.label().to_owned(),
                hash: block.hash(),
            },
        }
    }

    #[inline(always)]
    pub fn subject(&self) -> &String {
        &self.subject
    }

    #[inline(always)]
    pub fn block(&self) -> &String {
        &self.block.name
    }

    #[inline(always)]
    pub fn output(&self) -> &PathBuf {
        &self.output
    }
}
