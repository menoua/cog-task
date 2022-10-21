use std::fmt::Debug;
use thiserror::Error;

#[derive(Clone, Error, Debug)]
pub enum Error {
    #[error("Failed to load resource: {0}")]
    ResourceLoadError(String),
    #[error("Failed to load resource: {0}")]
    AudioDecodingError(String),
    #[error("Failed to decode stream: {0}")]
    StreamDecodingError(String),
    #[error("Invalid trigger configuration: {0}")]
    TriggerConfigError(String),
    #[error("Invalid resource: {0}")]
    InvalidResourceError(String),
    #[error("Invalid name format: {0}")]
    InvalidNameError(String),
    #[error("Failed to evolve action: {0}")]
    ActionEvolveError(String),
    #[error("Failed to update action: {0}")]
    ActionUpdateError(String),
    #[error("Failed to display action: {0}")]
    ActionViewError(String),
    #[error("I/O error: {0}")]
    IoAccessError(String),
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("Invalid task definition: {0}")]
    TaskDefinitionError(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfigError(String),
    #[error("Environment error: {0}")]
    EnvironmentError(String),
    #[error("Logger error: {0}")]
    LoggerError(String),
    #[error("Flow error: {0}")]
    FlowError(String),
    #[error("Graph error: {0}")]
    GraphError(String),
    #[error("Checksum error: {0}")]
    ChecksumError(String),
    #[error("Backend error: {0}")]
    BackendError(String),
}
