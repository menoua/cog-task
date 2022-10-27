use crate::action::Action;
use crate::comm::QWriter;
use crate::server::{config::LogFormat, AsyncSignal, Config, Info};
use chrono::{DateTime, Local};
use eyre::{eyre, Context, Error, Result};
use itertools::Itertools;
use ron::ser::PrettyConfig;
use serde::{Serialize, Serializer};
use serde_cbor::{from_slice, Value};
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

pub const TAG_INFO: u64 = 0x01;
pub const TAG_CONFIG: u64 = 0x02;
pub const TAG_ACTION: u64 = 0x03;

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
    Write(String, Value),
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
    pub fn new(info: &Info, config: &Config) -> Result<Self> {
        let block = normalized_name(info.block());
        let date = Local::now().format("%F").to_string();
        let time = Local::now().format("%T").to_string().replace(':', "-");
        let out_dir = info
            .output()
            .join(&info.subject())
            .join(format!("{date}/{block}/{time}"));

        if out_dir.exists() {
            return Err(eyre!("Output directory already exists: {out_dir:?}"));
        }
        create_dir_all(&out_dir)
            .wrap_err_with(|| format!("Failed to create output directory: {out_dir:?}"))?;

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

    fn write(&mut self, name: String, content: Value) -> Result<()> {
        let name = format!("{}.log", normalized_name(&name));
        let path = self.out_dir.join(name);
        let file = File::create(&path)
            .wrap_err_with(|| format!("Failed to create log file ({path:?})."))?;

        write_as(file, &content, self.log_format)?;

        #[cfg(debug_assertions)]
        println!("{:?} -> Wrote to file: {path:?}", Local::now());
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        for (group, (vec, flush)) in self.content.iter_mut().filter(|(_, (_, flush))| *flush) {
            let name = format!("{}.log", normalized_name(group));
            let path = self.out_dir.join(name);
            let mut file = File::create(&path)
                .wrap_err_with(|| format!("Failed to create log file ({path:?})."))?;

            write_vec(&mut file, self.log_format, vec)?;
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
    ) -> Result<()> {
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
            LoggerSignal::Write(name, content) => {
                self.write(name, content)?;
            }
            LoggerSignal::Flush => {
                self.flush()?;
                self.needs_flush = false;
            }
        }

        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        self.flush()
            .wrap_err("Failed to graciously close logger.")?;

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

fn write_vec(file: &mut File, fmt: LogFormat, vec: &Vec<(String, String, Value)>) -> Result<()> {
    let mut vec_t: Vec<(&str, &str, Serializable)> = vec![];
    for (a, b, v) in vec {
        vec_t.push((a, b, Serializable::try_from(v)?));
    }

    write_as(file, &vec_t, fmt)
}

fn write_as<W, T>(mut file: W, content: &T, fmt: LogFormat) -> Result<()>
where
    W: Write,
    T: ?Sized + Serialize,
{
    match fmt {
        LogFormat::JSON => {
            serde_json::to_writer_pretty(file, &content).wrap_err("Failed to log JSON to file")
        }
        LogFormat::YAML => {
            serde_yaml::to_writer(file, &content).wrap_err("Failed to log YAML to file")
        }
        LogFormat::RON => file
            .write_all(
                ron::ser::to_string_pretty(&content, PrettyConfig::default())
                    .wrap_err("Failed to serialize log as RON")?
                    .as_bytes(),
            )
            .wrap_err("Failed to log RON to file"),
    }
}

enum Serializable<'a> {
    Info(Info),
    Config(Config),
    Action(Box<dyn Action>),
    Value(&'a Value),
}

impl<'a> Serialize for Serializable<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Serializable::Info(info) => info.serialize(serializer),
            Serializable::Config(config) => config.serialize(serializer),
            Serializable::Action(action) => action.serialize(serializer),
            Serializable::Value(value) => value.serialize(serializer),
        }
    }
}

impl<'a> TryFrom<&'a Value> for Serializable<'a> {
    type Error = Error;

    fn try_from(v: &'a Value) -> Result<Self> {
        Ok(match v {
            Value::Tag(TAG_INFO, v) => Serializable::Info(match v.as_ref() {
                Value::Bytes(v) => from_slice::<Info>(v).unwrap(),
                _ => return Err(eyre!("Failed to deserialize Info struct")),
            }),
            Value::Tag(TAG_CONFIG, v) => Serializable::Config(match v.as_ref() {
                Value::Bytes(v) => from_slice::<Config>(v).unwrap(),
                _ => return Err(eyre!("Failed to deserialize Info struct",)),
            }),
            Value::Tag(TAG_ACTION, v) => Serializable::Action(match v.as_ref() {
                Value::Bytes(v) => from_slice::<Box<dyn Action>>(v).unwrap(),
                _ => return Err(eyre!("Failed to deserialize Info struct",)),
            }),
            v => Serializable::Value(v),
        })
    }
}
