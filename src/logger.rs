use crate::config::{Config, LogFormat};
use crate::error;
use crate::error::Error::LoggerError;
use crate::scheduler::info::Info;
use crate::scheduler::processor::AsyncSignal;
use crate::signal::QWriter;
use chrono::{DateTime, Local};
use itertools::Itertools;
use ron::ser::PrettyConfig;
use ron::Value;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::{create_dir_all, File};
use std::io::Write;
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
pub enum LoggerSignal {
    Append(String, (String, Value)),
    Extend(String, Vec<(String, Value)>),
    Flush,
}

impl LoggerSignal {
    #[inline(always)]
    fn requires_flush(&self) -> bool {
        matches!(
            self,
            LoggerSignal::Append(_, _) | LoggerSignal::Extend(_, _)
        )
    }
}

impl From<LoggerSignal> for AsyncSignal {
    fn from(signal: LoggerSignal) -> Self {
        AsyncSignal::Logger(Local::now(), signal)
    }
}

impl Logger {
    pub fn new(info: &Info, config: &Config) -> Result<Self, error::Error> {
        let block = normalized_name(info.block());
        let date = Local::now().format("%F").to_string();
        let time = Local::now().format("%T").to_string().replace(':', "-");
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

    fn append(&mut self, time: DateTime<Local>, group: String, entry: (String, Value)) {
        let time = time.to_string();
        let (name, value) = entry;
        let (vec, flush) = self.content.entry(group).or_default();
        vec.push((time, name, value));
        *flush = true;
        self.needs_flush = true;
    }

    fn extend(&mut self, time: DateTime<Local>, group: String, entries: Vec<(String, Value)>) {
        let time = time.to_string();
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
            let mut file = File::create(&path).map_err(|e| {
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
                LogFormat::RON => file
                    .write_all(
                        ron::ser::to_string_pretty(vec, PrettyConfig::default())
                            .map_err(|e| {
                                LoggerError(format!("Failed to serialize log to RON:\n{e:#?}"))
                            })?
                            .as_bytes(),
                    )
                    .map_err(|e| {
                        LoggerError(format!("Failed to log RON to file: {path:?}\n{e:#?}"))
                    })?,
            }
            *flush = false;

            #[cfg(debug_assertions)]
            println!("{:?} -> Wrote to file: {path:?}", Local::now());
        }
        self.needs_flush = false;
        Ok(())
    }

    pub fn update(
        &mut self,
        time: DateTime<Local>,
        signal: LoggerSignal,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        if signal.requires_flush() && !self.needs_flush {
            let mut async_writer = async_writer.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(5));
                async_writer.push(LoggerSignal::Flush);
            });
        }

        match signal {
            LoggerSignal::Append(group, entry) => {
                self.append(time, group, entry);
            }
            LoggerSignal::Extend(group, entries) => {
                self.extend(time, group, entries);
            }
            LoggerSignal::Flush => {
                self.flush()?;
                self.needs_flush = false;
            }
        }

        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), error::Error> {
        self.flush()
            .map_err(|e| LoggerError(format!("Failed to graciously close logger:\n{e:#?}")))?;

        self.content.clear();
        Ok(())
    }
}

pub fn normalized_name(name: &str) -> String {
    name.to_lowercase()
        .split_whitespace()
        .join("_")
        .replace('-', "_")
}
