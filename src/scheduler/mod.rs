use crate::action::audio::Audio;
use crate::action::counter::Counter;
use crate::action::{Action, ActionEnum, ActionSignal, StatefulActionEnum};
#[cfg(feature = "benchmark")]
use crate::benchmark::Profiler;
use crate::config::{Config, LogCondition};
use crate::error;
use crate::error::Error;
use crate::error::Error::{FlowError, InternalError, LoggerError};
use crate::io::IO;
use crate::logger::{Logger, LoggerSignal};
use crate::scheduler::info::Info;
use crate::scheduler::monitor::{Event, Monitor};
use crate::scheduler::processor::{AsyncProcessor, AsyncSignal, Atomic, SyncProcessor, SyncSignal};
use crate::server::{Server, ServerSignal};
use crate::signal::{QReader, QWriter};
use chrono::{DateTime, Local};
use eframe::egui;
use eframe::egui::{CentralPanel, Color32, CursorIcon, RichText};
use itertools::Itertools;
use num_traits::Zero;
use serde_json::Value;
use std::collections::HashSet;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

pub mod info;
pub mod monitor;
pub mod processor;

#[derive(Debug)]
pub struct Scheduler {
    tree: Arc<Mutex<StatefulActionEnum>>,
    info: Info,
    last_esc: Option<SystemTime>,
    config: Config,
    _io: IO,
    sync_writer: QWriter<SyncSignal>,
    async_writer: QWriter<AsyncSignal>,
    server_writer: QWriter<ServerSignal>,
    #[cfg(feature = "benchmark")]
    profiler: Profiler,
}

impl Scheduler {
    pub fn new(server: &Server, ctx: &egui::Context) -> Result<Self, error::Error> {
        let task = server.task();
        let block = server.active_block();
        let info = Info::new(server, task, block);
        let resources = server.resources();
        let config = block.config(server.config());
        let io = IO::new()?;
        let tree = block
            .action_tree()
            .inner()
            .stateful(0, resources, &config, &io)?;

        let server_writer = server.callback_channel();
        let mut async_writer = AsyncProcessor::spawn(&info, &config, &server_writer)?;
        let (mut sync_writer, tree) =
            SyncProcessor::spawn(ctx, tree, &async_writer, &server_writer)?;

        async_writer.push(LoggerSignal::Extend(
            "mainevent".to_owned(),
            vec![
                ("info".to_owned(), serde_json::to_value(&info).unwrap()),
                ("config".to_owned(), serde_json::to_value(&config).unwrap()),
            ],
        ));
        sync_writer.push(SyncSignal::UpdateGraph);

        Ok(Self {
            tree,
            info,
            last_esc: None,
            config,
            _io: io,
            sync_writer,
            async_writer,
            server_writer,
            #[cfg(feature = "benchmark")]
            profiler: Profiler::new(
                "Scheduler",
                vec!["keys", "proc", "show"],
                Duration::from_secs(60),
            ),
        })
    }

    #[inline(always)]
    pub fn info(&self) -> &Info {
        &self.info
    }

    #[inline(always)]
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn request_interrupt(&mut self) {
        self.async_writer.push(LoggerSignal::Append(
            "mainevent".to_owned(),
            (
                "interrupt".to_owned(),
                Value::String("User request".to_owned()),
            ),
        ));

        self.server_writer.push(ServerSignal::BlockInterrupted);
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Result<(), error::Error> {
        #[cfg(feature = "benchmark")]
        self.profiler.step();

        #[cfg(feature = "benchmark")]
        self.profiler.tic(0);
        if ui.input().key_pressed(egui::Key::Escape) {
            let time = SystemTime::now();
            if let Some(t) = self.last_esc.take() {
                if time.duration_since(t).unwrap() < Duration::from_millis(300) {
                    #[cfg(feature = "benchmark")]
                    self.profiler.toc(0);
                    return Ok(self.request_interrupt());
                }
            }
            self.last_esc = Some(time);
        }

        let mut keys_pressed = ui.input().keys_down.clone();
        keys_pressed.retain(|k| ui.input().key_pressed(*k));
        if !keys_pressed.is_empty() {
            self.sync_writer.push(SyncSignal::KeyPress(keys_pressed))
        }
        #[cfg(feature = "benchmark")]
        self.profiler.toc(0);

        #[cfg(feature = "benchmark")]
        self.profiler.tic(2);
        {
            let mut tree = self.tree.lock().unwrap();
            if tree.inner().props().visual() {
                let result = tree.inner_mut().show(ui, &mut self.sync_writer, &mut self.async_writer);

                if let Err(e) = &result {
                    self.async_writer.push(LoggerSignal::Append(
                        "mainevent".to_owned(),
                        ("crash".to_owned(), Value::String(format!("{e:#?}"))),
                    ));
                }

                if tree.inner().props().animated() {
                    ui.ctx().request_repaint();
                }

                #[cfg(feature = "benchmark")]
                self.profiler.toc(2);
                return result;
            }
        }
        #[cfg(feature = "benchmark")]
        self.profiler.toc(2);

        ui.output().cursor_icon = CursorIcon::None;

        Ok(())
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.sync_writer.push(SyncSignal::Finish);
        self.async_writer.push(AsyncSignal::Finish);
    }
}
