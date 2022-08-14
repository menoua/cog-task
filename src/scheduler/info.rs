use crate::assets::VERSION;
use crate::server::Server;
use crate::task::block::Block;
use crate::task::Task;
use crate::util::Hash;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize)]
pub struct Info {
    server: ServerInfo,
    task: TaskInfo,
    block: BlockInfo,
}

#[derive(Debug, Default, Serialize)]
pub struct TaskInfo {
    name: String,
    version: String,
    hash: String,
}

#[derive(Debug, Default, Serialize)]
pub struct BlockInfo {
    name: String,
    hash: String,
}

#[derive(Debug, Default, Serialize)]
pub struct ServerInfo {
    subject: String,
    output: PathBuf,
    version: String,
    hash: String,
}

impl Info {
    pub fn new(server: &Server, task: &Task, block: &Block) -> Self {
        Self {
            server: ServerInfo {
                subject: server.subject().to_owned(),
                output: server.env().output().to_owned(),
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
        &self.server.subject
    }

    #[inline(always)]
    pub fn block(&self) -> &String {
        &self.block.name
    }

    #[inline(always)]
    pub fn output(&self) -> &PathBuf {
        &self.server.output
    }
}
