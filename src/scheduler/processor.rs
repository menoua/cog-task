use crate::action::nil::StatefulNil;
use crate::action::{Action, ActionSignal, StatefulAction};
use crate::config::Config;
use crate::error;
use crate::io::IO;
use crate::logger::{Logger, LoggerSignal};
use crate::resource::ResourceMap;
use crate::scheduler::info::Info;
use crate::server::ServerSignal;
use crate::signal::{QReader, QWriter};
use chrono::{DateTime, Local};
use eframe::egui;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncSignal {
    UpdateGraph,
    KeyPress(HashSet<egui::Key>),
    Repaint,
    Finish,
}

pub struct SyncProcessor {
    ctx: egui::Context,
    tree: Arc<Mutex<Box<dyn StatefulAction>>>,
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
    ) -> Result<QWriter<AsyncSignal>, error::Error> {
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

            let result = proc.logger.finish();
            proc.server_writer.push(ServerSignal::AsyncComplete(result));
        });

        Ok(async_writer)
    }
}

impl SyncProcessor {
    pub fn spawn(
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        ctx: &egui::Context,
        tree: &dyn Action,
        async_writer: &QWriter<AsyncSignal>,
        server_writer: &QWriter<ServerSignal>,
    ) -> Result<(QWriter<SyncSignal>, Arc<Mutex<Box<dyn StatefulAction>>>), error::Error> {
        let sync_reader = QReader::new();
        let sync_writer = sync_reader.writer();
        let tree = tree.stateful(io, res, config, &sync_writer, async_writer)?;
        let mut proc = Self {
            ctx: ctx.clone(),
            tree: Arc::new(Mutex::new(tree)),
            sync_reader,
            sync_writer,
            async_writer: async_writer.clone(),
            server_writer: server_writer.clone(),
        };

        let sync_writer = proc.sync_writer.clone();
        let tree = proc.tree.clone();

        thread::spawn(move || {
            proc.sync_reader.pop();
            proc.start().unwrap_or_else(|e| {
                proc.server_writer.push(ServerSignal::BlockCrashed(e));
            });

            'mainloop: while let Some(signals) = proc.sync_reader.poll() {
                for signal in signals {
                    match signal {
                        SyncSignal::UpdateGraph => proc.tree.lock().unwrap().update(
                            &ActionSignal::UpdateGraph,
                            &mut proc.sync_writer,
                            &mut proc.async_writer,
                        ),
                        SyncSignal::KeyPress(keys) => proc.tree.lock().unwrap().update(
                            &ActionSignal::KeyPress(keys),
                            &mut proc.sync_writer,
                            &mut proc.async_writer,
                        ),
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

                    let mut tree = proc.tree.lock().unwrap();
                    if tree.is_over().unwrap_or_else(|e| {
                        proc.server_writer.push(ServerSignal::BlockCrashed(e));
                        let _ = tree.stop(&mut proc.sync_writer, &mut proc.async_writer);
                        *tree = Box::new(StatefulNil::new());
                        proc.ctx.request_repaint();
                        false
                    }) {
                        proc.server_writer.push(ServerSignal::BlockFinished);
                        tree.stop(&mut proc.sync_writer, &mut proc.async_writer)?;
                        *tree = Box::new(StatefulNil::new());
                        proc.ctx.request_repaint();
                    }
                }
            }

            proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
            proc.ctx.request_repaint();
            Result::<(), error::Error>::Ok(())
        });

        Ok((sync_writer, tree))
    }

    fn start(&mut self) -> Result<(), error::Error> {
        let tree = &mut (*self.tree.lock().unwrap());

        self.async_writer.push(LoggerSignal::Append(
            "mainevent".to_owned(),
            ("start".to_owned(), Value::String("Success".to_owned())),
        ));

        tree.start(&mut self.sync_writer, &mut self.async_writer)?;

        if tree.is_over()? {
            self.server_writer.push(ServerSignal::BlockFinished);
            self.ctx.request_repaint();
        }

        Ok(())
    }
}
