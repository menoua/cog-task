use crate::assets::{Icon, VERSION};
use crate::signal::QReader;
use crate::style;
use crate::style::text::{button1, tooltip};
use crate::style::{style_ui, Style, TEXT_SIZE_DIALOGUE_BODY, TEXT_SIZE_DIALOGUE_TITLE};
use crate::system::SystemInfo;
use eframe::egui;
use eframe::egui::{CursorIcon, Direction, Layout, Vec2, Window};
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
    sync_reader: QReader<LauncherSignal>,
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
                sync_reader: QReader::new(),
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
                sync_reader: QReader::new(),
            }
        }
    }

    pub fn window_size(&self) -> Vec2 {
        let count = self.task_paths.len() as u32;
        let width = 580;
        let height = (185 + count * 75).max(245).min(640);
        Vec2::from([width as f32, height as f32])
    }

    fn run_task(&mut self, task: PathBuf) {
        if task.file_name().is_none() || self.busy {
            return;
        }

        let curr = current_dir().unwrap();
        let root = current_exe().unwrap().parent().unwrap().to_path_buf();
        let path = root.join("bin").join("server").to_str().unwrap().to_owned();
        let mut sync_writer = self.sync_reader.writer();
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
                        sync_writer.push(LauncherSignal::TaskCrashed(stderr));
                    } else {
                        sync_writer.push(LauncherSignal::TaskClosed);
                    }
                }
                Err(e) => {
                    let status = format!(
                        "Failed to spawn server. Make sure it is located in \
                            bin/server relative to the launcher.\n{e:#?}"
                    );
                    println!("\nEE: {status}");
                    sync_writer.push(LauncherSignal::TaskCrashed(status));
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
            initial_window_size: Some(self.window_size() * 2.0),
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

    fn process(&mut self, msg: LauncherSignal) {
        match (self.busy, msg) {
            (true, LauncherSignal::TaskClosed) => {
                self.busy = false;
            }
            (true, LauncherSignal::TaskCrashed(status)) => {
                self.status = Status::Result(status);
                self.busy = false;
            }
            _ => {}
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LauncherSignal {
    TaskCrashed(String),
    TaskClosed,
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        while let Some(message) = self.sync_reader.try_pop() {
            self.process(message);
        }

        frame.set_window_size(self.window_size());

        self.show(ctx);

        ctx.set_pixels_per_point(2.0);
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}

impl Launcher {
    fn show(&mut self, ctx: &egui::Context) {
        let frame = egui::Frame::window(&ctx.style())
            .inner_margin(0.0)
            .outer_margin(0.0);

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            if self.busy {
                ui.output().cursor_icon = CursorIcon::NotAllowed;
            }

            ui.add_enabled_ui(!self.busy, |ui| {
                StripBuilder::new(ui)
                    .size(Size::exact(60.0))
                    .size(Size::exact(40.0))
                    .size(Size::exact(18.0))
                    .size(Size::exact(4.0))
                    .size(Size::exact(18.0))
                    .size(Size::remainder())
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            ui.centered_and_justified(|ui| {
                                ui.heading(if self.busy {
                                    format!("CogTask v{VERSION} (busy)")
                                } else {
                                    format!("CogTask v{VERSION}")
                                });
                            });
                        });

                        strip.cell(|ui| {
                            StripBuilder::new(ui)
                                .size(Size::remainder())
                                .size(Size::exact(240.0))
                                .size(Size::remainder())
                                .horizontal(|mut strip| {
                                    strip.empty();
                                    strip.cell(|ui| self.show_controls(ui));
                                    strip.empty();
                                });
                        });

                        strip.empty();

                        strip.strip(|builder| {
                            builder
                                .size(Size::remainder())
                                .size(Size::exact(240.0))
                                .size(Size::remainder())
                                .horizontal(|mut strip| {
                                    strip.empty();
                                    strip.cell(|ui| {
                                        ui.vertical_centered_justified(|ui| {
                                            ui.separator();
                                        });
                                    });
                                    strip.empty();
                                });
                        });

                        strip.empty();

                        strip.cell(|ui| self.show_tasks(ui));
                    });
            });
        });

        if !matches!(self.status, Status::None) {
            self.show_status(ctx);
        }
    }

    fn show_controls(&mut self, ui: &mut egui::Ui) {
        enum Interaction {
            None,
            LoadTask,
            LoadTaskRepo,
            ShowSystemInfo,
            ShowHelp,
        }

        let mut interaction = Interaction::None;

        style_ui(ui, Style::IconControls);
        ui.columns(4, |columns| {
            if columns[0]
                .button(Icon::Folder)
                .on_hover_text(tooltip("Load task"))
                .clicked()
            {
                interaction = Interaction::LoadTask;
            }
            if columns[1]
                .button(Icon::FolderTree)
                .on_hover_text(tooltip("Load task catalogue"))
                .clicked()
            {
                interaction = Interaction::LoadTaskRepo;
            }
            if columns[2]
                .button(Icon::SystemInfo)
                .on_hover_text(tooltip("System information"))
                .clicked()
            {
                interaction = Interaction::ShowSystemInfo;
            }
            if columns[3]
                .button(Icon::Help)
                .on_hover_text(tooltip("Help"))
                .clicked()
            {
                interaction = Interaction::ShowHelp;
            }
        });

        match interaction {
            Interaction::None => {}
            Interaction::LoadTask => {
                let path = FileDialog::new()
                    .set_location(current_dir().unwrap().to_str().unwrap())
                    .show_open_single_dir()
                    .unwrap();

                if let Some(path) = path {
                    self.run_task(path);
                }
            }
            Interaction::LoadTaskRepo => {
                let path = FileDialog::new()
                    .set_location(current_dir().unwrap().to_str().unwrap())
                    .show_open_single_dir()
                    .unwrap();

                if let Some(path) = path {
                    let sys_info = self.sys_info.clone();
                    *self = Self::new(path);
                    self.sys_info = sys_info;
                }
            }
            Interaction::ShowSystemInfo => {
                self.status = Status::SystemInfo;
            }
            Interaction::ShowHelp => {
                self.status = Status::Help;
            }
        }
    }

    fn show_tasks(&mut self, ui: &mut egui::Ui) {
        enum Interaction {
            None,
            StartTask(usize),
        }

        let mut interaction = Interaction::None;

        let task_buttons: Vec<_> = self
            .task_labels
            .iter()
            .map(|label| egui::Button::new(button1(label)))
            .collect();

        if task_buttons.is_empty() {
            ui.horizontal_centered(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label("(No tasks found in task directory)");
                });
            });
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    style_ui(ui, Style::SelectButton);
                    ui.spacing_mut().item_spacing = Vec2::new(25.0, 20.0);
                    for (i, button) in task_buttons.into_iter().enumerate() {
                        if ui.add(button).clicked() {
                            interaction = Interaction::StartTask(i);
                        }
                    }
                });
            });
        }

        match interaction {
            Interaction::None => {}
            Interaction::StartTask(i) => self.run_task(self.task_paths[i].clone()),
        }
    }

    fn show_status(&mut self, ctx: &egui::Context) {
        if matches!(self.status, Status::None) {
            return;
        }

        let (title, content) = match &self.status {
            Status::Result(status) => ("Status", status.to_owned()),
            Status::SystemInfo => ("System Info", format!("{:#?}", self.sys_info)),
            Status::Help => ("Help", "...".to_owned()),
            _ => ("", "".to_owned()),
        };

        let mut open = true;
        Window::new(
            RichText::from(title)
                .size(TEXT_SIZE_DIALOGUE_TITLE)
                .strong(),
        )
        .collapsible(false)
        .open(&mut open)
        .vscroll(true)
        .min_width(550.0)
        .default_size(Vec2::new(550.0, 240.0))
        .show(ctx, |ui| {
            ui.with_layout(
                Layout::centered_and_justified(Direction::LeftToRight),
                |ui| {
                    ui.label(RichText::from(content.clone()).size(TEXT_SIZE_DIALOGUE_BODY * 0.9));
                },
            )
            .response
            .context_menu(|ui| {
                if ui
                    .button(RichText::new("Copy").size(TEXT_SIZE_DIALOGUE_BODY * 0.9))
                    .clicked()
                {
                    ui.close_menu();
                    ui.output().copied_text = content.trim().to_owned();
                }
            });
        });
        if !open {
            self.status = Status::None;
        }
    }
}
