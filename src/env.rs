use crate::error;
use crate::error::Error::EnvironmentError;
use std::env::current_dir;
use std::path::PathBuf;

#[derive(Debug, Default, Clone)]
pub struct Env {
    root_dir: PathBuf,
    task_dir: PathBuf,
    output_dir: PathBuf,
    resource_dir: PathBuf,
}

impl Env {
    pub fn new(task_dir: PathBuf) -> Result<Self, error::Error> {
        let root_dir = current_dir()
            .map_err(|e| EnvironmentError(format!("Unable to get current directory:\n{e:#?}")))?;
        let task_name = task_dir.file_name().unwrap().to_str().unwrap().to_owned();

        let output_dir = root_dir.join("output").join(&task_name);
        if !output_dir.is_dir() {
            std::fs::create_dir_all(&output_dir).map_err(|e| {
                EnvironmentError(format!(
                    "Unable to create output directory: {output_dir:?}\n{e:#?}"
                ))
            })?;
        }

        let resource_dir = task_dir.join("data");

        Ok(Self {
            root_dir,
            task_dir,
            output_dir,
            resource_dir,
        })
    }

    #[inline]
    pub fn root(&self) -> &PathBuf {
        &self.root_dir
    }

    #[inline]
    pub fn task(&self) -> &PathBuf {
        &self.task_dir
    }

    #[inline]
    pub fn output(&self) -> &PathBuf {
        &self.output_dir
    }

    #[inline]
    pub fn resource(&self) -> &PathBuf {
        &self.resource_dir
    }
}
