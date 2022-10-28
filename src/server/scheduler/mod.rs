pub mod p_async;
pub mod p_sync;

pub use p_async::*;
pub use p_sync::*;

use crate::action::StatefulAction;
use crate::comm::QWriter;
use crate::resource::{LoggerSignal, IO, TAG_ACTION, TAG_CONFIG, TAG_INFO};
use crate::server::{Config, Info, Server, ServerSignal};
use eframe::egui;
use eframe::egui::{CentralPanel, CursorIcon, Frame};
use eyre::Result;
use serde_cbor::{ser::to_vec, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

pub type State = BTreeMap<u16, Value>;
pub type Atomic = Arc<Mutex<(Box<dyn StatefulAction>, State)>>;

pub struct Scheduler {
    atomic: Atomic,
    info: Info,
    last_esc: Option<SystemTime>,
    config: Config,
    _io: IO,
    ctx: egui::Context,
    sync_writer: QWriter<SyncSignal>,
    async_writer: QWriter<AsyncSignal>,
    server_writer: QWriter<ServerSignal>,
}

impl Scheduler {
    pub fn new(server: &Server, ctx: &egui::Context) -> Result<Self> {
        let task = server.task();
        let block = server.active_block().unwrap();
        let info = Info::new(server, task, block);
        let resources = server.resources();
        let config = block.config(server.config());
        let io = IO::new()?;
        let tree = block.action_tree();
        let state = block.default_state();

        let server_writer = server.callback_channel();
        let mut async_writer = AsyncProcessor::spawn(&info, &config, &server_writer)?;
        let (mut sync_writer, atomic) = SyncProcessor::spawn(
            &io,
            resources,
            &config,
            ctx,
            tree,
            state,
            &async_writer,
            &server_writer,
        )?;

        async_writer.push(LoggerSignal::Extend(
            "main".to_owned(),
            vec![
                (
                    "info".to_owned(),
                    Value::Tag(TAG_INFO, Box::new(Value::Bytes(to_vec(&info).unwrap()))),
                ),
                (
                    "config".to_owned(),
                    Value::Tag(TAG_CONFIG, Box::new(Value::Bytes(to_vec(&config).unwrap()))),
                ),
                (
                    "tree".to_owned(),
                    Value::Tag(TAG_ACTION, Box::new(Value::Bytes(block.action_tree_vec()))),
                ),
            ],
        ));
        sync_writer.push(SyncSignal::UpdateGraph);

        Ok(Self {
            atomic,
            info,
            last_esc: None,
            config,
            _io: io,
            ctx: ctx.clone(),
            sync_writer,
            async_writer,
            server_writer,
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
            "main".to_owned(),
            (
                "interrupt".to_owned(),
                Value::Text("user request".to_owned()),
            ),
        ));

        self.server_writer.push(ServerSignal::BlockInterrupted);
        self.ctx.request_repaint();
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Result<()> {
        if ui.input().key_pressed(egui::Key::Escape) {
            let time = SystemTime::now();
            if let Some(t) = self.last_esc.take() {
                if time.duration_since(t).unwrap() < Duration::from_millis(300) {
                    self.request_interrupt();
                    return Ok(());
                }
            }
            self.last_esc = Some(time);
        }

        let keys_pressed: BTreeSet<_> = ui
            .input()
            .keys_down
            .iter()
            .filter_map(|k| {
                if ui.input().key_pressed(*k) {
                    Some(k.into())
                } else {
                    None
                }
            })
            .collect();
        if !keys_pressed.is_empty() {
            self.sync_writer
                .push(SyncSignal::KeyPress(Instant::now(), keys_pressed))
        }

        let result = {
            let (tree, state) = &mut *self.atomic.lock().unwrap();
            CentralPanel::default()
                .frame(Frame::default().fill(self.config.background().into()))
                .show_inside(ui, |ui| {
                    if tree.props().visual() {
                        tree.show(ui, &mut self.sync_writer, &mut self.async_writer, state)
                    } else {
                        ui.output().cursor_icon = CursorIcon::None;
                        Ok(())
                    }
                })
                .inner
        };

        if let Err(e) = &result {
            self.async_writer.push(LoggerSignal::Append(
                "main".to_owned(),
                ("crash".to_owned(), Value::Text(format!("{e:#?}"))),
            ));
        }

        result
    }

    pub fn sync_writer(&mut self) -> &mut QWriter<SyncSignal> {
        &mut self.sync_writer
    }

    pub fn async_writer(&mut self) -> &mut QWriter<AsyncSignal> {
        &mut self.async_writer
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.async_writer.push(LoggerSignal::Append(
            "main".to_owned(),
            ("finish".to_owned(), Value::Text("ok".to_owned())),
        ));

        self.sync_writer.push(SyncSignal::Finish);
        self.async_writer.push(AsyncSignal::Finish);
    }
}
