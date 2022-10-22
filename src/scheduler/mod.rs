use crate::action::StatefulAction;
#[cfg(feature = "benchmark")]
use crate::benchmark::Profiler;
use crate::config::Config;
use crate::error;
use crate::io::IO;
use crate::logger::LoggerSignal;
use crate::scheduler::info::Info;
use crate::scheduler::processor::{AsyncProcessor, AsyncSignal, SyncProcessor, SyncSignal};
use crate::server::{Server, ServerSignal};
use crate::signal::QWriter;
use eframe::egui;
use eframe::egui::{CentralPanel, CursorIcon, Frame};
use ron::Value;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

pub mod info;
pub mod monitor;
pub mod processor;

pub struct Scheduler {
    tree: Arc<Mutex<Box<dyn StatefulAction>>>,
    info: Info,
    last_esc: Option<SystemTime>,
    config: Config,
    _io: IO,
    ctx: egui::Context,
    sync_writer: QWriter<SyncSignal>,
    async_writer: QWriter<AsyncSignal>,
    server_writer: QWriter<ServerSignal>,
    #[cfg(feature = "benchmark")]
    profiler: Profiler,
}

impl Scheduler {
    pub fn new(server: &Server, ctx: &egui::Context) -> Result<Self, error::Error> {
        let task = server.task();
        let block = server.active_block().unwrap();
        let info = Info::new(server, task, block);
        let resources = server.resources();
        let config = block.config(server.config());
        let io = IO::new()?;
        let tree = block.action_tree();
        println!("{tree:?}");

        let server_writer = server.callback_channel();
        let mut async_writer = AsyncProcessor::spawn(&info, &config, &server_writer)?;
        let (mut sync_writer, tree) = SyncProcessor::spawn(
            &io,
            &resources,
            &config,
            ctx,
            tree,
            &async_writer,
            &server_writer,
        )?;

        async_writer.push(LoggerSignal::Extend(
            "mainevent".to_owned(),
            vec![
                (
                    "info".to_owned(),
                    ron::to_string(&info).unwrap().parse().unwrap(),
                ),
                (
                    "config".to_owned(),
                    ron::to_string(&config).unwrap().parse().unwrap(),
                ),
            ],
        ));
        sync_writer.push(SyncSignal::UpdateGraph);

        Ok(Self {
            tree,
            info,
            last_esc: None,
            config,
            _io: io,
            ctx: ctx.clone(),
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
                Value::String("user request".to_owned()),
            ),
        ));

        self.server_writer.push(ServerSignal::BlockInterrupted);
        self.ctx.request_repaint();
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
            self.sync_writer
                .push(SyncSignal::KeyPress(Instant::now(), keys_pressed))
        }
        #[cfg(feature = "benchmark")]
        self.profiler.toc(0);

        #[cfg(feature = "benchmark")]
        self.profiler.tic(2);
        let result = {
            let mut tree = self.tree.lock().unwrap();
            CentralPanel::default()
                .frame(Frame::default().fill(self.config.background().into()))
                .show_inside(ui, |ui| {
                    if tree.props().visual() {
                        tree.show(ui, &mut self.sync_writer, &mut self.async_writer)
                    } else {
                        ui.output().cursor_icon = CursorIcon::None;
                        Ok(())
                    }
                })
                .inner
        };

        if let Err(e) = &result {
            self.async_writer.push(LoggerSignal::Append(
                "mainevent".to_owned(),
                ("crash".to_owned(), Value::String(format!("{e:#?}"))),
            ));
        }
        #[cfg(feature = "benchmark")]
        self.profiler.toc(2);

        result
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.async_writer.push(LoggerSignal::Append(
            "mainevent".to_owned(),
            ("finish".to_owned(), Value::String("ok".to_owned())),
        ));

        self.sync_writer.push(SyncSignal::Finish);
        self.async_writer.push(AsyncSignal::Finish);
    }
}
