mod activity;
mod cleanup;
mod loading;
mod selection;
mod startup;

#[cfg(feature = "benchmark")]
use crate::benchmark::Profiler;
use crate::config::Config;
use crate::env::Env;
use crate::queue::{QReader, QWriter};
use crate::resource::ResourceMap;
use crate::scheduler::Scheduler;
use crate::system::SystemInfo;
use crate::task::block::Block;
use crate::task::Task;
use crate::{error, style};
use chrono::NaiveDateTime;
use eframe::egui;
use eframe::egui::CentralPanel;
use eframe::glow::HasContext;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Page {
    Startup,
    Selection,
    Loading,
    Activity,
    CleanUp,
}

#[derive(Debug, Clone)]
pub enum Progress {
    None,
    Success,
    Interrupt,
    Failure(error::Error),
    CleanupError(error::Error),
    LastRun(NaiveDateTime),
}

pub struct Server {
    env: Env,
    task: Task,
    subject: String,
    scale_factor: f32,
    hold_on_rescale: bool,
    resources: ResourceMap,
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
    #[cfg(feature = "benchmark")]
    profiler: Profiler,
}

impl Server {
    pub fn new(path: PathBuf, bin_hash: String) -> anyhow::Result<Self> {
        let env = Env::new(path)?;
        let task = Task::new(env.task())?;
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
            scale_factor: 1.0,
            hold_on_rescale: false,
            resources: ResourceMap::new(),
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
            #[cfg(feature = "benchmark")]
            profiler: Profiler::new(
                "Server",
                vec!["fps", "proc", "show"],
                Duration::from_secs(60),
            ),
        })
    }

    pub fn run(mut self) {
        let options = eframe::NativeOptions {
            always_on_top: false,
            maximized: true,
            decorated: true,
            fullscreen: true,
            drag_and_drop_support: false,
            icon_data: None,
            initial_window_pos: None,
            initial_window_size: None,
            min_window_size: None,
            max_window_size: None,
            resizable: false,
            transparent: false,
            vsync: false,
            multisampling: 0,
            depth_buffer: 0,
            stencil_buffer: 0,
            hardware_acceleration: eframe::HardwareAcceleration::Preferred,
            renderer: Default::default(),
            follow_system_theme: false,
            default_theme: eframe::Theme::Light,
            run_and_return: false,
        };

        self.sys_info.renderer = format!("{:#?}", options.renderer);
        self.sys_info.hw_acceleration = format!("{:#?}", options.hardware_acceleration);

        eframe::run_native(
            &self.title(),
            options,
            Box::new(|cc| {
                style::init(&cc.egui_ctx);
                if let Some(gl) = &cc.gl {
                    self.sys_info
                        .renderer
                        .push_str(&format!(" ({:?})", gl.version()))
                }
                Box::new(self)
            }),
        );
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
    pub fn resources(&self) -> &ResourceMap {
        &self.resources
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

    fn process(&mut self, ctx: &egui::Context, signal: ServerSignal) {
        match (self.page, signal) {
            (Page::Loading, ServerSignal::LoadComplete) => match Scheduler::new(self, ctx) {
                Ok(scheduler) => {
                    self.page = Page::Activity;
                    self.scheduler = Some(scheduler);
                }
                Err(e) => {
                    self.sync_reader.push(ServerSignal::BlockCrashed(e));
                }
            },
            (Page::Loading, ServerSignal::BlockCrashed(e)) => {
                self.status = Progress::Failure(e);
                self.page = Page::Selection;
                self.cleaning_up = 0;
            }
            (Page::Activity, ServerSignal::BlockFinished) => {
                self.status = Progress::Success;
                self.drop_scheduler();
            }
            (Page::Activity, ServerSignal::BlockInterrupted) => {
                self.status = Progress::Interrupt;
                self.drop_scheduler();
            }
            (Page::Activity, ServerSignal::BlockCrashed(e)) => {
                self.status = Progress::Failure(e);
                self.drop_scheduler();
            }
            (Page::CleanUp, ServerSignal::SyncComplete(success))
            | (Page::CleanUp, ServerSignal::AsyncComplete(success)) => {
                self.cleaning_up -= 1;
                if self.cleaning_up == 0 {
                    self.blocks.get_mut(self.active_block.unwrap()).unwrap().1 =
                        match (&self.status, success) {
                            (Progress::Success, Err(e)) => {
                                self.status = Progress::CleanupError(e.clone());
                                Progress::CleanupError(e)
                            }
                            (progress, _) => progress.clone(),
                        };

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
        #[cfg(feature = "benchmark")]
        {
            self.profiler.report();
            self.profiler.reset();
            println!("Block ended...");
        }

        self.page = Page::CleanUp;
        self.cleaning_up = 2;
        self.scheduler.take();
    }
}

#[derive(Debug, Clone)]
pub enum ServerSignal {
    LoadComplete,
    BlockFinished,
    BlockInterrupted,
    BlockCrashed(error::Error),
    SyncComplete(Result<(), error::Error>),
    AsyncComplete(Result<(), error::Error>),
}

impl eframe::App for Server {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(feature = "benchmark")]
        self.profiler.step();

        #[cfg(feature = "benchmark")]
        {
            self.profiler.toc(0);
            self.profiler.tic(0);
        }

        #[cfg(feature = "benchmark")]
        self.profiler.tic(1);
        while let Some(signal) = self.sync_reader.try_pop() {
            self.process(ctx, signal);
        }
        #[cfg(feature = "benchmark")]
        self.profiler.toc(1);

        let frame = egui::Frame::window(&ctx.style())
            .inner_margin(0.0)
            .outer_margin(0.0);

        #[cfg(feature = "benchmark")]
        self.profiler.tic(2);
        CentralPanel::default()
            .frame(frame)
            .show(ctx, |ui| match self.page {
                Page::Startup => self.show_startup(ui),
                Page::Selection => self.show_selection(ui),
                Page::Activity => self.show_activity(ui),
                Page::Loading => self.show_loading(ui),
                Page::CleanUp => self.show_cleanup(ui),
            });
        #[cfg(feature = "benchmark")]
        self.profiler.toc(2);

        if !self.hold_on_rescale {
            style::set_fullscreen_scale(ctx, self.scale_factor);
        }
        if !matches!(self.page, Page::Activity) {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }
}
