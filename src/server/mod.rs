pub mod env;
pub mod info;
pub mod page;
pub mod scheduler;
pub mod task;

pub use env::Env;
pub use info::*;
pub use page::*;
pub use scheduler::*;
pub use task::*;

use crate::comm::{QReader, QWriter};
use crate::gui;
use crate::resource::LoggerSignal;
use crate::util::SystemInfo;
use chrono::{DateTime, Local, NaiveDateTime};
use eframe::egui::CentralPanel;
use eframe::glow::HasContext;
use eframe::{egui, App};
use eyre::{eyre, Context, Error, Result};
use serde_cbor::Value;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug)]
pub enum Progress {
    None,
    Success(DateTime<Local>),
    Interrupt(DateTime<Local>),
    Failure(DateTime<Local>, Error),
    CleanupError(DateTime<Local>, Error),
    LastRun(NaiveDateTime),
}

pub struct Server {
    env: Env,
    task: Task,
    subject: String,
    scale_factor: u32,
    hold_on_rescale: bool,
    scheduler: Option<Scheduler>,
    page: Page,
    blocks: Vec<(String, Progress)>,
    active_block: Option<usize>,
    status: Progress,
    show_magnification: bool,
    bin_hash: String,
    sys_info: SystemInfo,
    sync_reader: QReader<ServerSignal>,
    cleaning_up: u32,
}

impl Server {
    pub fn new(path: PathBuf, bin_hash: String) -> Result<Self> {
        let env = Env::new(path)?;
        let task = Task::new(env.task())
            .wrap_err_with(|| format!("Failed to start task ({:?}).", env.task()))?;
        let blocks = task
            .block_labels()
            .into_iter()
            .map(|label| (label, Progress::None))
            .collect();

        println!("Saving output to: {:?}", env.output());

        Ok(Self {
            env,
            task,
            subject: "".to_owned(),
            scale_factor: 100,
            hold_on_rescale: false,
            scheduler: None,
            page: Page::Startup,
            blocks,
            active_block: None,
            status: Progress::None,
            show_magnification: false,
            bin_hash,
            sys_info: SystemInfo::new(),
            sync_reader: QReader::new(),
            cleaning_up: 0,
        })
    }

    pub fn run(mut self) -> Result<()> {
        let options = eframe::NativeOptions {
            always_on_top: false,
            maximized: true,
            decorated: true,
            fullscreen: true,
            // fullsize_content: true, //TODO this is exclusive to macOS
            drag_and_drop_support: false,
            icon_data: None,
            initial_window_pos: None,
            initial_window_size: None,
            min_window_size: None,
            max_window_size: None,
            resizable: true,
            transparent: false,
            mouse_passthrough: false,
            vsync: false,
            multisampling: 0,
            depth_buffer: 0,
            stencil_buffer: 0,
            hardware_acceleration: eframe::HardwareAcceleration::Preferred,
            renderer: Default::default(),
            follow_system_theme: false,
            default_theme: eframe::Theme::Light,
            run_and_return: false,
            event_loop_builder: None, // look into this argument at some point
            shader_version: None,     // look into this argument at some point
            centered: true,
            ..Default::default()
        };

        self.sys_info.renderer = format!("{:#?}", options.renderer);
        self.sys_info.hw_acceleration = format!("{:#?}", options.hardware_acceleration);

        eframe::run_native(
            &self.title(),
            options,
            Box::new(|cc| {
                gui::init(&cc.egui_ctx);
                if let Some(gl) = &cc.gl {
                    self.sys_info
                        .renderer
                        .push_str(&format!(" ({:?})", gl.version()))
                }
                Box::new(self)
            }),
        )
        .map_err(|e| eyre!("Failed to run native eframe: {e}"))
    }

    #[inline(always)]
    fn title(&self) -> String {
        format!("CogTask Server -- {}", self.task.title())
    }

    #[inline(always)]
    pub fn env(&self) -> &Env {
        &self.env
    }

    #[inline(always)]
    pub fn subject(&self) -> &String {
        &self.subject
    }

    #[inline(always)]
    pub fn active_block(&self) -> Option<&Block> {
        self.active_block.map(|i| self.task.block(i))
    }

    #[inline(always)]
    pub fn config(&self) -> &Config {
        self.task.config()
    }

    pub fn style(&self) -> Option<Vec<u8>> {
        let path = self.env.task().join("style.css");
        if path.exists() {
            Some(
                std::fs::read(&path).unwrap_or_else(|_| {
                    panic!("Failed to read custom task styling file: {path:?}")
                }),
            )
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn task(&self) -> &Task {
        &self.task
    }

    fn process(&mut self, _ctx: &egui::Context, signal: ServerSignal) {
        match (self.page, signal) {
            (Page::Loading, ServerSignal::LoadComplete) => {
                if let Some(scheduler) = self.scheduler.as_mut() {
                    self.page = Page::Activity;
                    scheduler.sync_writer().push(SyncSignal::Go);
                } else {
                    self.page = Page::Selection;
                }
            }
            (Page::Loading, ServerSignal::BlockCrashed(e)) => {
                self.status = Progress::Failure(Local::now(), e);
                self.drop_scheduler();
            }
            (Page::Activity, ServerSignal::BlockFinished) => {
                self.status = Progress::Success(Local::now());
                self.drop_scheduler();
            }
            (Page::Activity, ServerSignal::BlockInterrupted) => {
                self.status = Progress::Interrupt(Local::now());
                self.drop_scheduler();
            }
            (Page::Activity, ServerSignal::BlockCrashed(e)) => {
                if let Some(scheduler) = self.scheduler.as_mut() {
                    scheduler.async_writer().push(LoggerSignal::Write(
                        "crash".to_owned(),
                        Value::Text(format!("{e:?}")),
                    ));
                }
                self.status = Progress::Failure(Local::now(), e);
                self.drop_scheduler();
            }
            (Page::CleanUp, ServerSignal::SyncComplete(success))
            | (Page::CleanUp, ServerSignal::AsyncComplete(success)) => {
                self.cleaning_up -= 1;
                if self.cleaning_up == 0 {
                    if let (Progress::Success(_), Err(e)) = (&self.status, success) {
                        self.status = Progress::CleanupError(Local::now(), e);
                    }
                    self.page = Page::Selection;
                }
            }
            _ => {}
        };
    }

    #[inline(always)]
    pub(crate) fn callback_channel(&self) -> QWriter<ServerSignal> {
        self.sync_reader.writer()
    }

    fn valid_subject_id(&self) -> bool {
        !self.subject.is_empty()
            && self
                .subject
                .chars()
                .all(|c| c.is_alphabetic() || c.is_alphanumeric() | "-_".contains(c))
    }

    #[inline(always)]
    pub fn hash(&self) -> String {
        self.bin_hash.clone()
    }

    fn drop_scheduler(&mut self) {
        self.page = Page::CleanUp;
        self.cleaning_up = 2;
        self.scheduler.take();
    }
}

#[derive(Debug)]
pub enum ServerSignal {
    LoadComplete,
    BlockFinished,
    BlockInterrupted,
    BlockCrashed(Error),
    SyncComplete(Result<()>),
    AsyncComplete(Result<()>),
}

impl App for Server {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Some(signal) = self.sync_reader.try_pop() {
            self.process(ctx, signal);
        }

        let frame = egui::Frame::window(&ctx.style())
            .inner_margin(0.0)
            .outer_margin(0.0);

        CentralPanel::default()
            .frame(frame)
            .show(ctx, |ui| match self.page {
                Page::Startup => self.show_startup(ui),
                Page::Selection => self.show_selection(ui),
                Page::Activity => self.show_activity(ui),
                Page::Loading => self.show_loading(ui),
                Page::CleanUp => self.show_cleanup(ui),
            });

        if !self.hold_on_rescale {
            gui::set_fullscreen_scale(ctx, self.scale_factor as f32 / 100.0);
        }
        if !matches!(self.page, Page::Activity) {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }
}
