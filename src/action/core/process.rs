use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{IoManager, LoggerSignal, ResourceAddr, ResourceManager, ResourceValue};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eyre::{eyre, Context, Error, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvError, TryRecvError};
use std::thread;
use std::time::Instant;

#[derive(Debug, Deserialize, Serialize)]
pub struct Process {
    #[serde(default)]
    name: String,
    src: PathBuf,
    #[serde(default)]
    vars: BTreeMap<String, Value>,
    #[serde(default = "defaults::on_start")]
    on_start: bool,
    #[serde(default = "defaults::on_change")]
    on_change: bool,
    #[serde(default)]
    once: bool,
    #[serde(default = "defaults::blocking")]
    blocking: bool,
    #[serde(default)]
    in_mapping: BTreeMap<SignalId, String>,
    #[serde(default)]
    in_update: SignalId,
    lo_incoming: SignalId,
    #[serde(default)]
    out_result: SignalId,
}

stateful!(Process {
    name: String,
    vars: BTreeMap<String, Value>,
    on_start: bool,
    on_change: bool,
    once: bool,
    blocking: bool,
    in_mapping: BTreeMap<SignalId, String>,
    in_update: SignalId,
    lo_incoming: SignalId,
    out_result: SignalId,
    child: Child,
    stdin: ChildStdin,
    link: Receiver<Response>,
});

mod defaults {
    pub fn on_start() -> bool {
        true
    }

    pub fn on_change() -> bool {
        true
    }

    pub fn blocking() -> bool {
        true
    }
}

enum Response {
    Result(Value),
    Error(Error),
    End,
}

impl Action for Process {
    fn init(self) -> Result<Box<dyn Action>>
    where
        Self: 'static + Sized,
    {
        if self.lo_incoming == 0 {
            return Err(eyre!("`lo_incoming`for Process cannot be zero."));
        }

        Ok(Box::new(self))
    }

    fn in_signals(&self) -> BTreeSet<SignalId> {
        let mut signals: BTreeSet<_> = self.in_mapping.keys().cloned().collect();
        signals.extend([self.in_update, self.lo_incoming]);
        signals
    }

    fn out_signals(&self) -> BTreeSet<SignalId> {
        BTreeSet::from([self.lo_incoming, self.out_result])
    }

    fn resources(&self, _config: &Config) -> Vec<ResourceAddr> {
        vec![ResourceAddr::Ref(self.src.clone())]
    }

    fn stateful(
        &self,
        _io: &IoManager,
        res: &ResourceManager,
        _config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        let src = match res.fetch(&ResourceAddr::Ref(self.src.clone()))? {
            ResourceValue::Ref(src) => src,
            _ => return Err(eyre!("Resource address and value types don't match.")),
        };

        let mut child = Command::new(src)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .wrap_err("Failed to spawn child process.")?;

        let stdin = child
            .stdin
            .take()
            .ok_or(eyre!("Failed to open stdin of child process."))?;

        let stdout = child
            .stdout
            .take()
            .ok_or(eyre!("Failed to open stdout of child process."))?;

        let (tx, rx) = mpsc::channel();

        let lo_incoming = self.lo_incoming;
        let mut sync_writer = sync_writer.clone();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);

            loop {
                let mut response = String::with_capacity(1024);
                if let Err(e) = reader.read_line(&mut response) {
                    sync_writer.push(SyncSignal::Error(eyre!(
                        "Failed to receive response from child process:\n{e:#?}"
                    )));
                    break;
                }

                let response = response.strip_suffix('\n').unwrap();
                let (typ, value) = match response.split_once(' ') {
                    Some(pair) => pair,
                    None => (response, ""),
                };

                let response = match typ {
                    "nil" => Response::Result(Value::Null),
                    "true" => Response::Result(Value::Bool(true)),
                    "false" => Response::Result(Value::Bool(false)),
                    "i64" => value.parse::<i128>().map_or_else(
                        |e| {
                            Response::Error(eyre!(
                                "Failed to parse (claimed) i64 response from child process:\n{e:?}"
                            ))
                        },
                        |v| Response::Result(Value::Integer(v)),
                    ),
                    "f64" => value.parse::<f64>().map_or_else(
                        |e| {
                            Response::Error(eyre!(
                                "Failed to parse (claimed) f64 response from child process:\n{e:?}"
                            ))
                        },
                        |v| Response::Result(Value::Float(v)),
                    ),
                    "str" => Response::Result(Value::Text(value.replace("\\n", "\n"))),
                    "err" => Response::Error(eyre!(value.replace("\\n", "\n"))),
                    "end" => Response::End,
                    _ => {
                        Response::Error(eyre!("Unknown response type ({typ}) from child process."))
                    }
                };

                let end = matches!(response, Response::End | Response::Error(_));

                if tx.send(response).is_err() {
                    break;
                }
                sync_writer.push(SyncSignal::Emit(
                    Instant::now(),
                    Signal::from(vec![(lo_incoming, Value::Null)]),
                ));
                if end {
                    break;
                }
            }
        });

        Ok(Box::new(StatefulProcess {
            done: false,
            name: self.name.clone(),
            vars: self.vars.clone(),
            on_start: self.on_start,
            on_change: self.on_change,
            once: self.once,
            blocking: self.blocking,
            in_mapping: BTreeMap::new(),
            in_update: self.in_update,
            lo_incoming: self.lo_incoming,
            out_result: self.out_result,
            child,
            stdin,
            link: rx,
        }))
    }
}

impl StatefulAction for StatefulProcess {
    impl_stateful!();

    fn props(&self) -> Props {
        if self.once { DEFAULT } else { INFINITE }.into()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        for (id, var) in self.in_mapping.iter() {
            if let Some(entry) = self.vars.get_mut(var) {
                if let Some(value) = state.get(id) {
                    *entry = value.clone();
                }
            }
        }

        if self.on_start {
            if self.once && self.blocking {
                self.done = true;
                sync_writer.push(SyncSignal::UpdateGraph);
            }

            self.run(sync_writer, async_writer)
                .wrap_err("Failed to evaluate function.")
        } else {
            Ok(Signal::none())
        }
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        let mut news: Vec<(SignalId, Value)> = vec![];
        let mut changed = false;
        let mut updated = false;
        if let ActionSignal::StateChanged(_, signal) = signal {
            for id in signal {
                if let Some(var) = self.in_mapping.get(id) {
                    if let Some(entry) = self.vars.get_mut(var) {
                        *entry = state.get(id).unwrap().clone();
                    }
                    changed = true;
                }

                if *id == self.lo_incoming {
                    let result = match self.link.try_recv() {
                        Ok(Response::Result(v)) => v,
                        Ok(Response::Error(e)) => {
                            return Err(eyre!("Child process returned error:\n{e:#?}"));
                        }
                        Ok(Response::End) => {
                            self.done = true;
                            sync_writer.push(SyncSignal::UpdateGraph);
                            return Ok(Signal::none());
                        }
                        Err(TryRecvError::Empty) => continue,
                        Err(TryRecvError::Disconnected) => {
                            return Err(eyre!("Child process died without informing about it."));
                        }
                    };
                    self.vars.insert("self".to_owned(), result.clone());

                    if !self.name.is_empty() {
                        async_writer.push(LoggerSignal::Append(
                            "math".to_owned(),
                            (self.name.clone(), result.clone()),
                        ));
                    }

                    if self.out_result > 0 {
                        news.push((self.out_result, result.clone()));
                    }

                    if self.once {
                        self.done = true;
                        sync_writer.push(SyncSignal::UpdateGraph);
                    }
                }
            }

            if signal.contains(&self.in_update) {
                updated = true;
            }
        }

        if (changed && self.on_change) || updated {
            news.extend(
                self.run(sync_writer, async_writer)
                    .wrap_err("Failed to run process.")?,
            );
        }

        Ok(news.into())
    }

    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        // self.stdin
        //     .write_all("stop".as_bytes())
        //     .wrap_err("Failed to stop child process.")?;
        let _ = self.child.kill();
        Ok(Signal::none())
    }
}

impl StatefulProcess {
    #[inline(always)]
    fn run(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<Signal> {
        let mut inputs = format!("with {}\n", self.vars.len());
        for (name, value) in self.vars.iter() {
            let value = match value {
                Value::Null => "nil".to_owned(),
                Value::Bool(true) => "true".to_owned(),
                Value::Bool(false) => "false".to_owned(),
                Value::Integer(i) => format!("i64 {i}"),
                Value::Float(f) => format!("f64 {f}"),
                Value::Text(s) => format!("str {}", s.replace('\n', "\\n")),
                v => return Err(eyre!("Cannot send value ({v:?}) to child process.")),
            };

            inputs.push_str(&format!("{name} {value}\n"));
        }
        inputs.push_str("go\n");

        self.stdin
            .write_all(inputs.as_bytes())
            .wrap_err("Failed to run child process step.")?;

        let mut news = vec![];
        if self.blocking {
            let result = match self.link.recv() {
                Ok(Response::Result(v)) => v,
                Ok(Response::Error(e)) => {
                    return Err(eyre!("Child process returned error:\n{e:#?}"));
                }
                Ok(Response::End) => {
                    self.done = true;
                    sync_writer.push(SyncSignal::UpdateGraph);
                    return Ok(Signal::none());
                }
                Err(RecvError) => {
                    return Err(eyre!("Child process died without informing about it."))
                }
            };
            self.vars.insert("self".to_owned(), result.clone());

            if !self.name.is_empty() {
                async_writer.push(LoggerSignal::Append(
                    "math".to_owned(),
                    (self.name.clone(), result.clone()),
                ));
            }

            if self.out_result > 0 {
                news.push((self.out_result, result));
            }

            if self.once {
                self.done = true;
                sync_writer.push(SyncSignal::UpdateGraph);
            }
        }

        Ok(news.into())
    }
}
