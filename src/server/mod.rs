mod activity;
mod cleanup;
mod loading;
mod selection;
mod startup;

use crate::config::Config;
use crate::env::Env;
use crate::message::MessageBuffer;
use crate::resource::ResourceMap;
use crate::scheduler::{Scheduler, SchedulerMsg};
use crate::system::SystemInfo;
use crate::task::block::Block;
use crate::task::Task;
use crate::{error, style};
use eframe::egui;
use eframe::egui::{CentralPanel, Rect, Sense};
use eframe::glow::HasContext;
use egui_extras::{Size, StripBuilder};
use iced::{window, Button};
use std::path::PathBuf;
use std::time::Duration;

const MIN_UPDATE_DELAY: Duration = Duration::from_millis(2);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Page {
    Startup,
    Selection,
    Loading,
    Activity,
    CleanUp,
}

pub enum Status {
    None,
    Success(String),
    Failure(anyhow::Error),
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
    blocks: Vec<(String, bool)>,
    _needs_refresh: bool,
    active_block: Option<usize>,
    capture_key: bool,
    capture_fps: Option<f64>,
    animation_id: u32,
    status: Option<Result<String, error::Error>>,
    show_magnification: bool,
    bin_hash: String,
    sys_info: SystemInfo,
    buffer: MessageBuffer<ServerMsg>,
}

impl Server {
    pub fn new(path: PathBuf, bin_hash: String) -> anyhow::Result<Self> {
        let env = Env::new(path)?;
        let task = Task::new(env.task())?;
        let blocks = task
            .block_labels()
            .into_iter()
            .map(|label| (label, false))
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
            _needs_refresh: false,
            active_block: None,
            capture_key: false,
            capture_fps: None,
            animation_id: 0,
            status: None,
            show_magnification: false,
            bin_hash,
            sys_info: SystemInfo::new(),
            buffer: MessageBuffer::new(),
        })
    }

    pub fn run(mut self) {
        let options = eframe::NativeOptions {
            always_on_top: true,
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
            vsync: true,
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
    pub fn active_block(&self) -> &Block {
        self.task.block(self.active_block.unwrap())
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

    fn process(&mut self, message: ServerMsg) {
        match (self.page, message) {
            (Page::Selection, ServerMsg::StartBlock(_i)) => {
                // if self.scheduler.is_none() {
                //     println!("\nStarting experiment block {i}...");
                //     self.active_block = Some(i);
                //     self.page = Page::Loading;
                //     // Command::perform(
                //     //     async {
                //     //         thread::sleep(Duration::from_millis(500));
                //     //     },
                //     //     move |()| ServerMsg::LoadResources(i),
                //     // );
                // }
            }
            (Page::Loading, ServerMsg::LoadResources(_i)) => {
                // if self.scheduler.is_none() {
                //     let block = self.task.block(i);
                //
                //     if let Err(e) =
                //         self.resources
                //             .preload_block(block, &self.env, self.task.config())
                //     {
                //         self.buffer
                //             .push_sync(Destination::default(), ServerMsg::CrashBlock(e));
                //         return;
                //     }
                //
                //     match Scheduler::new(self) {
                //         Ok((scheduler, _)) => {
                //             self.scheduler = Some(scheduler);
                //             // Command::batch([
                //             //     cmd,
                //             //     Command::perform(async {}, |()| ServerMsg::LoadComplete),
                //             // ]);
                //         }
                //         Err(e) => {
                //             self.buffer
                //                 .push_sync(Destination::default(), ServerMsg::CrashBlock(e));
                //         }
                //     }
                // }
            }
            (Page::Loading, ServerMsg::LoadComplete) => {
                // let at_least_until = Instant::now() + MIN_UPDATE_DELAY;
                // match self.scheduler.as_mut().unwrap().start() {
                //     Ok(cmd) => {
                //         self.page = Page::Activity;
                //         self.capture_key = true;
                //
                //         let now = Instant::now();
                //         if now < at_least_until {
                //             SpinSleeper::new(SPIN_DURATION)
                //                 .with_spin_strategy(SPIN_STRATEGY)
                //                 .sleep(at_least_until - now);
                //         }
                //
                //         // cmd
                //     }
                //     Err(e) => {
                //         self.buffer
                //             .push_sync(Destination::default(), ServerMsg::CrashBlock(e));
                //     }
                // }
            }
            (Page::Activity, ServerMsg::FinishBlock) => {
                // self.status = Some(Ok("Success".to_owned()));
                // self.page = Page::CleanUp;
                // self.capture_key = false;
                // self.capture_fps = None;
                // self.animation_id = 0;
                // thread::sleep(MIN_UPDATE_DELAY);
                // Command::perform(
                //     async move {
                //         thread::sleep(Duration::from_millis(500));
                //     },
                //     |()| ServerMsg::DropScheduler,
                // )
            }
            (Page::Activity, ServerMsg::InterruptBlock) => {
                // self.status = Some(Ok("Interrupted".to_owned()));
                // self.page = Page::CleanUp;
                // self.capture_key = false;
                // self.capture_fps = None;
                // self.animation_id = 0;
                // thread::sleep(MIN_UPDATE_DELAY);
                // Command::perform(
                //     async move {
                //         thread::sleep(Duration::from_millis(500));
                //     },
                //     |()| ServerMsg::DropScheduler,
                // )
            }
            (Page::Loading | Page::Activity, ServerMsg::CrashBlock(_e)) => {
                // self.status = Some(Err(e.clone()));
                // self.page = Page::CleanUp;
                // self.capture_key = false;
                // self.capture_fps = None;
                // self.animation_id = 0;
                //
                // if let Some(scheduler) = &mut self.scheduler {
                //     let _ = scheduler.update(SchedulerMsg::Logger(LoggerMsg::Append(
                //         "mainevent".to_owned(),
                //         ("crash".to_owned(), Value::String(format!("{e:#?}"))),
                //     )));
                // }
                //
                // thread::sleep(MIN_UPDATE_DELAY);
                // Command::perform(
                //     async move {
                //         thread::sleep(Duration::from_millis(500));
                //     },
                //     |()| ServerMsg::DropScheduler,
                // )
            }
            (Page::Loading | Page::Activity, ServerMsg::Relay(_msg)) => {
                // if let Some(scheduler) = self.scheduler.as_mut() {
                //     let at_least_until = Instant::now() + MIN_UPDATE_DELAY;
                //     match scheduler.update(msg) {
                //         Ok(cmd) => {
                //             // cmd
                //         }
                //         Err(e) => {
                //             self.buffer
                //                 .push_sync(Destination::default(), ServerMsg::CrashBlock(e));
                //         }
                //     }
                // } else {
                //     #[cfg(debug_assertions)]
                //     println!("WW: Tried to send message to non-existent scheduler");
                // }
            }
            (Page::CleanUp, ServerMsg::DropScheduler) => {
                // if let Some(mut scheduler) = self.scheduler.take() {
                //     match scheduler.stop() {
                //         Ok(cmd) => {
                //             // cmd
                //         }
                //         Err(e) => {
                //             self.buffer
                //                 .push_sync(Destination::default(), ServerMsg::CleanUp(Err(e)));
                //         }
                //     }
                // } else {
                //     self.buffer
                //         .push_sync(Destination::default(), ServerMsg::CleanUp(Ok(())));
                // }
            }
            (Page::CleanUp, ServerMsg::CleanUp(_success)) => {
                // match (&self.status, success) {
                //     (Some(Ok(status)), Ok(_)) if status.as_str() == "Success" => {
                //         self.blocks.get_mut(self.active_block.unwrap()).unwrap().1 = true;
                //     }
                //     (Some(Ok(status)), Err(e)) if status.as_str() == "Success" => {
                //         self.status = Some(Err(e));
                //     }
                //     _ => {}
                // }
                //
                // self.page = Page::Selection;
            }
            _ => {}
        };
    }

    #[inline(always)]
    fn mode(&self) -> window::Mode {
        window::Mode::Fullscreen
    }

    #[inline(always)]
    fn scale_factor(&self) -> f64 {
        self.scale_factor as f64
    }

    fn valid_subject_id(&self) -> bool {
        !self.subject.is_empty()
            && self
                .subject
                .chars()
                .all(|c| c.is_alphabetic() || c.is_alphanumeric() | "-_".contains(c))
    }

    pub fn hash(&self) -> String {
        self.bin_hash.clone()
    }
}

#[derive(Debug, Clone)]
pub enum ServerMsg {
    StartBlock(usize),                 // Ui
    LoadResources(usize),              // Ui
    LoadComplete,                      // Callback
    FinishBlock,                       // Callback
    InterruptBlock,                    // Callback
    CrashBlock(error::Error),          // Callback
    DropScheduler,                     // Ui
    CleanUp(Result<(), error::Error>), // Callback
    Relay(SchedulerMsg),               // Callback
}

impl eframe::App for Server {
    // fn subscription(&self) -> Subscription<Self::Message> {
    //     if self.capture_key {
    //         iced_native::subscription::events_with(|event, _status| match event {
    //             Event::Keyboard(KeyPressed { key_code, .. }) => {
    //                 Some(SchedulerMsg::KeyPress(key_code).wrap())
    //             }
    //             _ => None,
    //         })
    //     } else {
    //         Subscription::none()
    //     }
    // }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        while let Some((_dest, message)) = self.buffer.pop_sync() {
            self.process(message);
        }

        match self.page {
            Page::Startup => self.show_startup(ctx),
            Page::Selection => self.show_selection(ctx),
            Page::Activity => self.show_activity(ctx),
            Page::Loading => self.show_loading(ctx),
            Page::CleanUp => self.show_cleanup(ctx),
        }

        if !self.hold_on_rescale {
            style::set_fullscreen_scale(ctx, self.scale_factor);
        }
        if matches!(self.page, Page::Activity) {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
    }
}
