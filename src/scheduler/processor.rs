use crate::action::nil::StatefulNil;
use crate::action::{Action, ActionSignal};
use crate::config::Config;
use crate::io::IO;
use crate::logger::{Logger, LoggerSignal};
use crate::queue::{QReader, QWriter};
use crate::resource::ResourceMap;
use crate::scheduler::info::Info;
use crate::scheduler::{Atomic, State};
use crate::server::ServerSignal;
use crate::signal::Signal;
use chrono::{DateTime, Local};
use eframe::egui;
use eyre::Result;
use serde_cbor::Value;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum AsyncSignal {
    Logger(DateTime<Local>, LoggerSignal),
    Finish,
}

pub struct AsyncProcessor {
    logger: Logger,
    async_reader: QReader<AsyncSignal>,
    async_writer: QWriter<AsyncSignal>,
    server_writer: QWriter<ServerSignal>,
}

#[derive(Debug, Clone)]
pub enum SyncSignal {
    UpdateGraph,
    KeyPress(Instant, HashSet<egui::Key>),
    Emit(Instant, Signal),
    // External(Instant, Signal),
    Repaint,
    Finish,
}

pub struct SyncProcessor {
    ctx: egui::Context,
    atomic: Atomic,
    sync_reader: QReader<SyncSignal>,
    sync_writer: QWriter<SyncSignal>,
    async_writer: QWriter<AsyncSignal>,
    server_writer: QWriter<ServerSignal>,
}

impl AsyncProcessor {
    pub fn spawn(
        info: &Info,
        config: &Config,
        server_writer: &QWriter<ServerSignal>,
    ) -> Result<QWriter<AsyncSignal>> {
        let async_reader = QReader::new();
        let async_writer = async_reader.writer();
        let mut proc = Self {
            logger: Logger::new(info, config)?,
            async_reader,
            async_writer,
            server_writer: server_writer.clone(),
        };

        let async_writer = proc.async_writer.clone();

        thread::spawn(move || {
            while let Some(signal) = proc.async_reader.pop() {
                match signal {
                    AsyncSignal::Logger(time, signal) => {
                        proc.logger
                            .update(time, signal, &proc.async_writer)
                            .unwrap();
                    }
                    AsyncSignal::Finish => break,
                }
            }

            proc.server_writer
                .push(ServerSignal::AsyncComplete(proc.logger.finish()));
        });

        Ok(async_writer)
    }
}

impl PartialEq for SyncSignal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SyncSignal::UpdateGraph, SyncSignal::UpdateGraph) => true,
            (SyncSignal::KeyPress(t1, _), SyncSignal::KeyPress(t2, _)) => t1 == t2,
            (SyncSignal::Emit(_, _), SyncSignal::Emit(_, _)) => false,
            // External(Instant, Signal),
            (SyncSignal::Repaint, SyncSignal::Repaint) => true,
            (SyncSignal::Finish, SyncSignal::Finish) => true,
            _ => false,
        }
    }
}

impl Eq for SyncSignal {}

impl SyncProcessor {
    pub fn spawn(
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        ctx: &egui::Context,
        tree: &dyn Action,
        async_writer: &QWriter<AsyncSignal>,
        server_writer: &QWriter<ServerSignal>,
    ) -> Result<(QWriter<SyncSignal>, Atomic)> {
        let sync_reader = QReader::new();
        let sync_writer = sync_reader.writer();
        let tree = tree.stateful(io, res, config, &sync_writer, async_writer)?;
        let atomic = Arc::new(Mutex::new((tree, State::new())));
        let mut proc = Self {
            ctx: ctx.clone(),
            atomic,
            sync_reader,
            sync_writer,
            async_writer: async_writer.clone(),
            server_writer: server_writer.clone(),
        };

        let sync_writer = proc.sync_writer.clone();
        let atomic = proc.atomic.clone();

        thread::spawn(move || {
            proc.sync_reader.pop();
            proc.start().unwrap_or_else(|e| {
                proc.server_writer.push(ServerSignal::BlockCrashed(e));
            });

            'mainloop: while let Some(signals) = proc.sync_reader.poll() {
                for signal in signals {
                    match signal {
                        SyncSignal::UpdateGraph => {
                            let (tree, state) = &mut *proc.atomic.lock().unwrap();
                            tree.update(
                                &ActionSignal::UpdateGraph,
                                &mut proc.sync_writer,
                                &mut proc.async_writer,
                                state,
                            )
                        }
                        SyncSignal::KeyPress(time, keys) => {
                            let (tree, state) = &mut *proc.atomic.lock().unwrap();
                            tree.update(
                                &ActionSignal::KeyPress(time, keys),
                                &mut proc.sync_writer,
                                &mut proc.async_writer,
                                state,
                            )
                        }
                        SyncSignal::Emit(time, signal) => {
                            let (int_sig, _ext_sig, state_sig) = signal.split();
                            let (tree, state) = &mut *proc.atomic.lock().unwrap();

                            if !state_sig.is_empty() {
                                for (k, v) in state_sig.into_iter() {
                                    state.insert(k, v);
                                }

                                tree.update(
                                    &ActionSignal::StateChanged,
                                    &mut proc.sync_writer,
                                    &mut proc.async_writer,
                                    state,
                                )?;
                            }

                            if !int_sig.is_empty() {
                                tree.update(
                                    &ActionSignal::Internal(time, int_sig),
                                    &mut proc.sync_writer,
                                    &mut proc.async_writer,
                                    state,
                                )?;
                            }

                            Ok(())
                        }
                        SyncSignal::Repaint => {
                            proc.ctx.request_repaint();
                            Ok(())
                        }
                        SyncSignal::Finish => break 'mainloop,
                    }
                    .unwrap_or_else(|e| {
                        proc.server_writer.push(ServerSignal::BlockCrashed(e));
                        proc.ctx.request_repaint();
                    });

                    let (tree, state) = &mut *proc.atomic.lock().unwrap();
                    let is_over = tree.is_over().unwrap_or_else(|e| {
                        proc.server_writer.push(ServerSignal::BlockCrashed(e));
                        let _ = tree.stop(&mut proc.sync_writer, &mut proc.async_writer, state);
                        *tree = Box::new(StatefulNil::new());
                        proc.ctx.request_repaint();
                        false
                    });

                    if is_over {
                        proc.server_writer.push(ServerSignal::BlockFinished);
                        tree.stop(&mut proc.sync_writer, &mut proc.async_writer, state)?;
                        *tree = Box::new(StatefulNil::new());
                        proc.ctx.request_repaint();
                    }
                }
            }

            proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
            proc.ctx.request_repaint();
            Result::<()>::Ok(())
        });

        Ok((sync_writer, atomic))
    }

    fn start(&mut self) -> Result<()> {
        let (tree, state) = &mut *self.atomic.lock().unwrap();

        self.async_writer.push(LoggerSignal::Append(
            "main".to_owned(),
            ("start".to_owned(), Value::Text("ok".to_owned())),
        ));

        tree.start(&mut self.sync_writer, &mut self.async_writer, state)?;

        if tree.is_over()? {
            self.server_writer.push(ServerSignal::BlockFinished);
            self.ctx.request_repaint();
        }

        Ok(())
    }
}
