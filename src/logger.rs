use crate::config::{Config, LogFormat};
use crate::error;
use crate::error::Error::LoggerError;
use crate::scheduler::{info::Info, SchedulerMsg};
use crate::server::ServerMsg;
use iced::Command;
use itertools::Itertools;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::{create_dir_all, File};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

pub type LogGroup = (Vec<(String, String, Value)>, bool);

#[derive(Debug, Default)]
pub struct Logger {
    out_dir: PathBuf,
    content: HashMap<String, LogGroup>,
    needs_flush: bool,
    log_format: LogFormat,
}

#[derive(Debug, Clone)]
pub enum LoggerMsg {
    Append(String, (String, Value)),
    Extend(String, Vec<(String, Value)>),
    Flush,
    Finish,
}

impl LoggerMsg {
    #[inline(always)]
    fn requires_flush(&self) -> bool {
        matches!(self, LoggerMsg::Append(_, _) | LoggerMsg::Extend(_, _))
    }

    #[inline(always)]
    pub fn wrap(self) -> ServerMsg {
        ServerMsg::Relay(SchedulerMsg::Logger(self))
    }
}

impl Logger {
    pub fn new(info: &Info, config: &Config) -> Result<Self, error::Error> {
        let block = normalized_name(info.block());
        let date = chrono::Local::now().format("%F").to_string();
        let time = chrono::Local::now()
            .format("%T")
            .to_string()
            .replace(':', "-");
        let out_dir = info
            .output()
            .join(&info.subject())
            .join(format!("{date}/{block}/{time}"));

        if out_dir.exists() {
            Err(LoggerError(format!(
                "Output directory already exists: {out_dir:?}"
            )))?;
        }
        create_dir_all(&out_dir).map_err(|e| {
            LoggerError(format!(
                "Failed to create output directory: {out_dir:?}\n{e:#?}"
            ))
        })?;

        Ok(Self {
            out_dir,
            content: HashMap::new(),
            needs_flush: false,
            log_format: config.log_format(),
        })
    }

    fn append(&mut self, group: String, entry: (String, Value)) {
        let time = chrono::Local::now().to_string();
        let (name, value) = entry;
        let (vec, flush) = self.content.entry(group).or_default();
        vec.push((time, name, value));
        *flush = true;
        self.needs_flush = true;
    }

    fn extend(&mut self, group: String, entries: Vec<(String, Value)>) {
        let time = chrono::Local::now().to_string();
        let (vec, flush) = self.content.entry(group).or_default();
        vec.extend(
            entries
                .into_iter()
                .map(|(name, value)| (time.clone(), name, value)),
        );
        *flush = true;
        self.needs_flush = true;
    }

    fn flush(&mut self) -> Result<(), error::Error> {
        for (group, (vec, flush)) in self.content.iter_mut().filter(|(_, (_, flush))| *flush) {
            let name = format!("{}.log", normalized_name(group));
            let path = self.out_dir.join(name);
            let file = File::create(&path).map_err(|e| {
                LoggerError(format!(
                    "Failed to create log file for group `{group}`:\n{e:#?}"
                ))
            })?;

            match self.log_format {
                LogFormat::JSON => serde_json::to_writer_pretty(file, vec).map_err(|e| {
                    LoggerError(format!("Failed to log JSON to file: {path:?}\n{e:#?}"))
                })?,
                LogFormat::YAML => serde_yaml::to_writer(file, vec).map_err(|e| {
                    LoggerError(format!("Failed to log YAML to file: {path:?}\n{e:#?}"))
                })?,
            }
            *flush = false;

            #[cfg(debug_assertions)]
            println!("{:?} -> Wrote to file: {path:?}", chrono::Local::now());
        }
        self.needs_flush = false;
        Ok(())
    }

    pub fn update(&mut self, msg: LoggerMsg) -> Result<Command<ServerMsg>, error::Error> {
        let cmd = if msg.requires_flush() && !self.needs_flush {
            Command::perform(
                async {
                    thread::sleep(Duration::from_secs(30));
                    LoggerMsg::Flush
                },
                LoggerMsg::wrap,
            )
        } else {
            Command::none()
        };

        match msg {
            LoggerMsg::Append(group, entry) => {
                self.append(group, entry);
            }
            LoggerMsg::Extend(group, entries) => {
                self.extend(group, entries);
            }
            LoggerMsg::Flush => {
                self.flush()?;
                self.needs_flush = false;
            }
            LoggerMsg::Finish => {
                self.flush().map_err(|e| {
                    LoggerError(format!("Failed to graciously close logger:\n{e:#?}"))
                })?;
                self.content.clear();
            }
        }

        Ok(cmd)
    }
}

pub fn normalized_name(name: &str) -> String {
    name.to_lowercase()
        .split_whitespace()
        .join("_")
        .replace('-', "_")
}
