use crate::comm::{QReader, QWriter};
use crate::resource::{Logger, LoggerSignal};
use crate::server::{Config, Info, ServerSignal};
use chrono::{DateTime, Local};
use eyre::Result;
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
