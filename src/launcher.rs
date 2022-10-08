use crate::assets::{Icon, PIXELS_PER_POINT, VERSION};
use crate::message::{Destination, MessageBuffer};
use crate::style::{style_ui, Style};
use crate::system::SystemInfo;
use eframe::egui;
use eframe::egui::{Color32, CursorIcon, Vec2, Window};
use eframe::glow::HasContext;
use egui::widget_text::RichText;
use egui_extras::{Size, StripBuilder};
use heck::ToTitleCase;
use itertools::Itertools;
use native_dialog::FileDialog;
use std::env::{current_dir, current_exe};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

enum Status {
    None,
    Result(String),
    SystemInfo,
    Help,
}

pub struct Launcher {
    _root_dir: PathBuf,
    task_paths: Vec<PathBuf>,
    task_labels: Vec<String>,
    busy: bool,
    active_task: Option<String>,
    status: Status,
    sys_info: SystemInfo,
    buffer: MessageBuffer<LauncherMsg>,
}

impl Default for Launcher {
    fn default() -> Self {
        let root_dir = current_exe()
            .expect("Unable to get current directory.")
            .parent()
            .unwrap()
            .to_path_buf()
            .join("task");

        Self::new(root_dir)
    }
}

impl Launcher {
    pub fn new(root_dir: PathBuf) -> Self {
        if let Ok(content) = root_dir.read_dir() {
            let task_paths: Vec<_> = content
                .into_iter()
                .filter_map(|e| {
                    if let Ok(e) = e {
                        if let Ok(t) = e.file_type() {
                            if t.is_dir()
                                && ["json", "yml"]
                                    .into_iter()
                                    .any(|ext| e.path().join(format!("task.{ext}")).exists())
                            {
                                return Some(e.path());
                            }
                        }
                    }
                    None
                })
                .sorted()
                .collect();

            let task_labels: Vec<_> = task_paths
                .iter()
                .map(|p| p.file_name().unwrap().to_str().unwrap().to_title_case())
                .collect();

            Self {
                _root_dir: root_dir,
                task_paths,
                task_labels,
                busy: false,
                active_task: None,
                status: Status::None,
                sys_info: SystemInfo::new(),
                buffer: MessageBuffer::new(),
            }
        } else {
            Self {
                _root_dir: root_dir,
                task_paths: vec![],
                task_labels: vec![],
                busy: false,
                active_task: None,
                status: Status::None,
                sys_info: SystemInfo::new(),
                buffer: MessageBuffer::new(),
            }
        }
    }

    pub fn window_size(&self) -> Vec2 {
        let count = self.task_paths.len() as u32;
        let width = 230;
        let height = match count {
            0 => 90,
            1 => 120,
            2 => 135,
            3 => 160,
            4 => 180,
            _ => 210,
        };

        Vec2::from([width as f32, height as f32])
    }

    fn run_task(&mut self, task: PathBuf) {
        if task.file_name().is_none() || self.busy {
            return;
        }

        let curr = current_dir().unwrap();
        let root = current_exe().unwrap().parent().unwrap().to_path_buf();
        let path = root.join("bin").join("server").to_str().unwrap().to_owned();
        let mut buffer = self.buffer.clone();
        self.busy = true;
        self.active_task = Some(task.file_name().unwrap().to_str().unwrap().to_title_case());
        thread::spawn(move || {
            use std::process::Command;
            let proc = Command::new(path).current_dir(curr).arg(task).output();

            match proc {
                Ok(o) => {
                    let stdout = o.stdout.into_iter().map(|c| c as char).collect::<String>();
                    let stderr = o.stderr.into_iter().map(|c| c as char).collect::<String>();
                    if !stdout.is_empty() {
                        println!("\n{stdout}");
                    }
                    if !stderr.is_empty() {
                        eprintln!("\n{stderr}");
                        buffer.push_sync(Destination::default(), LauncherMsg::TaskCrash(stderr));
                    } else {
                        buffer.push_sync(Destination::default(), LauncherMsg::TaskClose);
                    }
                }
                Err(e) => {
                    let status = format!(
                        "Failed to spawn server. Make sure it is located in \
                            bin/server relative to the launcher.\n{e:#?}"
                    );
                    println!("\nEE: {status}");
                    buffer.push_sync(Destination::default(), LauncherMsg::TaskCrash(status));
                }
            }
        });
    }

    #[inline(always)]
    pub fn title() -> &'static str {
        "CogTask Launcher"
    }

    pub fn run(mut self) {
        let options = eframe::NativeOptions {
            always_on_top: false,
            maximized: false,
            decorated: true,
            fullscreen: false,
            drag_and_drop_support: false,
            icon_data: None,
            initial_window_pos: None,
            initial_window_size: Some(self.window_size() * PIXELS_PER_POINT / 2.0),
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
            Self::title(),
            options,
            Box::new(|cc| {
                crate::assets::setup(&cc.egui_ctx);
                if let Some(gl) = &cc.gl {
                    self.sys_info
                        .renderer
                        .push_str(&format!(" ({:?})", gl.version()))
                }
                Box::new(self)
            }),
        );
    }

    fn consume_sync_buffer(&mut self) {
        while let Some((dst, msg)) = self.buffer.pop_sync() {
            if dst.is_empty() {
                self.process(msg);
            }
        }
    }

    fn process(&mut self, msg: LauncherMsg) {
        match (self.busy, msg) {
            (true, LauncherMsg::TaskClose) => {
                self.busy = false;
            }
            (true, LauncherMsg::TaskCrash(status)) => {
                self.status = Status::Result(status);
                self.busy = false;
            }
            (_, LauncherMsg::ToClipboard) => {
                match &self.status {
                    Status::Result(_status) => {
                        // ui.output().copied_text = format!(
                        //     "Task \"{}\" failed with error:\n{status}",
                        //     self.active_task.as_ref().unwrap_or(&"[NONE]".to_owned())
                        // );
                    }
                    Status::SystemInfo => {
                        // ui.output().copied_text = format!("{:#?}", self.sys_info);
                    }
                    _ => {}
                }
            }
            _ => {}
        };
    }
}

#[derive(Debug, Clone)]
pub enum LauncherMsg {
    TaskCrash(String),
    TaskClose,
    ToClipboard,
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.consume_sync_buffer();

        frame.set_window_size(self.window_size());

        let ctrl_buttons: Vec<_> = vec![
            egui::Button::new(Icon::Folder),
            egui::Button::new(Icon::FolderTree),
            egui::Button::new(Icon::SystemInfo),
            egui::Button::new(Icon::Help),
        ];

        let task_buttons: Vec<_> = self
            .task_labels
            .iter()
            .map(|label| egui::Button::new(label))
            .collect();

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.busy {
                ui.output().cursor_icon = CursorIcon::NotAllowed;
            }

            ui.add_enabled_ui(!self.busy, |ui| {
                StripBuilder::new(ui)
                    .size(Size::exact(20.0))
                    .size(Size::exact(15.0))
                    .size(Size::exact(10.0))
                    .size(Size::remainder())
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            ui.centered_and_justified(|ui| {
                                ui.label(
                                    RichText::new(if self.busy {
                                        format!("CogTask v{VERSION} (busy)")
                                    } else {
                                        format!("CogTask v{VERSION}")
                                    })
                                    .color(Color32::BLACK)
                                    .heading(),
                                );
                            });
                        });

                        strip.cell(|ui| {
                            StripBuilder::new(ui)
                                .size(Size::remainder())
                                .size(Size::exact(80.0))
                                .size(Size::remainder())
                                .horizontal(|mut strip| {
                                    strip.empty();
                                    strip.cell(|ui| {
                                        style_ui(ui, Style::LauncherControls);
                                        ui.columns(ctrl_buttons.len(), |columns| {
                                            ctrl_buttons
                                                .into_iter()
                                                .enumerate()
                                                .filter_map(|(i, button)| {
                                                    let response =
                                                        columns[i].add(button).on_hover_text(
                                                            RichText::from(match i {
                                                                0 => "Load task",
                                                                1 => "Load task catalogue",
                                                                2 => "System information",
                                                                3 => "Help",
                                                                _ => "",
                                                            })
                                                            .size(9.0),
                                                        );
                                                    if response.clicked() {
                                                        Some(i)
                                                    } else {
                                                        None
                                                    }
                                                })
                                                .collect_vec()
                                        })
                                        .into_iter()
                                        .for_each(|i| {
                                            match i {
                                                0 => {
                                                    let path = FileDialog::new()
                                                        .set_location(
                                                            current_dir()
                                                                .unwrap()
                                                                .to_str()
                                                                .unwrap(),
                                                        )
                                                        .show_open_single_dir()
                                                        .unwrap();

                                                    if let Some(path) = path {
                                                        self.run_task(path);
                                                    }
                                                }
                                                1 => {
                                                    let path = FileDialog::new()
                                                        .set_location(
                                                            current_dir()
                                                                .unwrap()
                                                                .to_str()
                                                                .unwrap(),
                                                        )
                                                        .show_open_single_dir()
                                                        .unwrap();

                                                    if let Some(path) = path {
                                                        *self = Self::new(path);
                                                    }
                                                }
                                                2 => self.status = Status::SystemInfo,
                                                3 => self.status = Status::Help,
                                                _ => {}
                                            }
                                        });
                                    });
                                    strip.empty();
                                });
                        });

                        strip.empty();

                        strip.cell(|ui| {
                            if task_buttons.is_empty() {
                                ui.horizontal_centered(|ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.label("(No tasks found in task directory)");
                                    });
                                });
                            } else {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    ui.vertical_centered(|ui| {
                                        style_ui(ui, Style::LauncherSelection);
                                        for (i, button) in task_buttons.into_iter().enumerate() {
                                            if ui.add(button).clicked() {
                                                self.run_task(self.task_paths[i].clone());
                                            }
                                        }
                                    });
                                });
                            }
                        });
                    });
            });
        });

        if !matches!(self.status, Status::None) {
            let mut open = true;
            match &self.status {
                Status::Result(status) => {
                    Window::new(RichText::from("Status").size(14.0).strong())
                        .collapsible(false)
                        .open(&mut open)
                        .vscroll(true)
                        .show(ctx, |ui| {
                            ui.label(RichText::from(status).size(12.0).color(Color32::BLACK));
                        });
                }
                Status::SystemInfo => {
                    Window::new(RichText::from("System Info").size(14.0).strong())
                        .collapsible(false)
                        .open(&mut open)
                        .vscroll(true)
                        .show(ctx, |ui| {
                            ui.label(
                                RichText::from(format!("{:#?}", self.sys_info))
                                    .size(12.0)
                                    .color(Color32::BLACK),
                            );
                        });
                }
                Status::Help => {
                    Window::new(RichText::from("Help").size(14.0).strong())
                        .collapsible(false)
                        .open(&mut open)
                        .vscroll(true)
                        .show(ctx, |ui| {
                            ui.label(RichText::from("...").size(12.0).color(Color32::BLACK));
                        });
                }
                Status::None => {}
            }
            if !open {
                self.status = Status::None;
            }
        }

        ctx.request_repaint_after(Duration::from_millis(200));
    }
}
