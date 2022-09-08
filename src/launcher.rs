use crate::assets::{Icon, TEXT_LARGE, TEXT_TINY, TEXT_XSMALL, VERSION};
use crate::style;
use crate::style::CUSTOM_RED;
use heck::ToTitleCase;
use iced::alignment::{Horizontal, Vertical};
use iced::pure::widget::{
    tooltip::Position, Button, Column, Container, Row, Scrollable, Space, Text,
};
use iced::pure::{button, column, text, tooltip, Application, Element};
use iced::{Alignment, Color, Command, Length, Renderer};
use iced_aw::pure::{Card, Modal};
use itertools::Itertools;
use native_dialog::FileDialog;
use std::env::{current_dir, current_exe};
use std::path::PathBuf;

#[derive(Default, Debug)]
struct SystemInfo {
    sys_name: String,
    sys_kernel: String,
    sys_version: String,
    cpu_brand: String,
    cpu_cores: String,
    memory_total: String,
    memory_used: String,
    graphics_adapter: String,
    graphics_backend: String,
}

pub struct Launcher {
    _root_dir: PathBuf,
    task_paths: Vec<PathBuf>,
    task_labels: Vec<String>,
    busy: bool,
    active_task: Option<String>,
    status_message: Option<String>,
    show_system_info: bool,
    show_help: bool,
    sys_info: SystemInfo,
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
                status_message: None,
                show_system_info: false,
                show_help: false,
                sys_info: SystemInfo::default(),
            }
        } else {
            Self {
                _root_dir: root_dir,
                task_paths: vec![],
                task_labels: vec![],
                busy: false,
                active_task: None,
                status_message: None,
                show_system_info: false,
                show_help: false,
                sys_info: Default::default(),
            }
        }
    }

    pub fn window_size(&self) -> (u32, u32) {
        let count = self.task_paths.len() as u32;
        let width = 600;
        let height = match count {
            0 => 275,
            1 => 350,
            2 => 400,
            3 => 475,
            4 => 575,
            _ => 625,
        };

        (width, height)
    }

    fn run_task(&mut self, task: PathBuf) -> Command<LauncherMsg> {
        if task.file_name().is_none() {
            return Command::none();
        }

        let curr = current_dir().unwrap();
        let root = current_exe().unwrap().parent().unwrap().to_path_buf();
        let path = root.join("bin").join("server").to_str().unwrap().to_owned();
        self.busy = true;
        self.active_task = Some(task.file_name().unwrap().to_str().unwrap().to_title_case());
        Command::perform(
            async move {
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
                            LauncherMsg::TaskCrash(stderr)
                        } else {
                            LauncherMsg::TaskClose
                        }
                    }
                    Err(e) => {
                        let status = format!(
                            "Failed to spawn server. Make sure it is located in \
                            bin/server relative to the launcher.\n{e:#?}"
                        );
                        println!("\nEE: {status}");
                        LauncherMsg::TaskCrash(status)
                    }
                }
            },
            |msg| msg,
        )
    }
}

#[derive(Debug, Clone)]
pub enum LauncherMsg {
    StartTask(usize),
    TaskCrash(String),
    TaskClose,
    ToClipboard,
    CloseCard,
    LoadTask,
    LoadTaskRepo,
    ShowSystemInfo,
    // SystemInfoReceived(system::Information),
    ShowHelp,
}

impl Application for Launcher {
    type Executor = iced::executor::Default;
    type Message = LauncherMsg;
    type Flags = ();

    #[inline(always)]
    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Launcher::default(),
            Command::none(), // system::fetch_information(LauncherMsg::SystemInfoReceived),
        )
    }

    #[inline(always)]
    fn title(&self) -> String {
        "CogTask Launcher".to_owned()
    }

    fn update(&mut self, message: Self::Message) -> Command<LauncherMsg> {
        match (self.busy, message) {
            (false, LauncherMsg::LoadTask) => {
                let path = FileDialog::new()
                    .set_location(current_dir().unwrap().to_str().unwrap())
                    .show_open_single_dir()
                    .unwrap();

                match path {
                    Some(path) => self.run_task(path),
                    None => Command::none(),
                }
            }
            (false, LauncherMsg::LoadTaskRepo) => {
                let path = FileDialog::new()
                    .set_location(current_dir().unwrap().to_str().unwrap())
                    .show_open_single_dir()
                    .unwrap();

                match path {
                    Some(path) => {
                        *self = Self::new(path);
                        let (width, height) = self.window_size();
                        iced::window::resize(width, height)
                    }
                    None => Command::none(),
                }
            }
            (false, LauncherMsg::ShowSystemInfo) => {
                self.show_system_info = !self.show_system_info;
                Command::none()
            }
            // (false, LauncherMsg::SystemInfoReceived(information)) => {
            //     self.system_info.system_name = format!(
            //         "System name: {}",
            //         information
            //             .system_name
            //             .as_ref()
            //             .unwrap_or(&"unknown".to_owned())
            //     );
            //
            //     self.system_info.system_kernel = format!(
            //         "System kernel: {}",
            //         information
            //             .system_kernel
            //             .as_ref()
            //             .unwrap_or(&"unknown".to_owned())
            //     );
            //
            //     self.system_info.system_version = format!(
            //         "System version: {}",
            //         information
            //             .system_version
            //             .as_ref()
            //             .unwrap_or(&"unknown".to_owned())
            //     );
            //
            //     self.system_info.cpu_brand = format!("Processor brand: {}", information.cpu_brand);
            //
            //     self.system_info.cpu_cores = format!(
            //         "Processor cores: {}",
            //         information
            //             .cpu_cores
            //             .map_or("unknown".to_owned(), |cores| cores.to_string())
            //     );
            //
            //     self.system_info.memory_readable =
            //         ByteSize::kb(information.memory_total).to_string();
            //
            //     self.system_info.memory_total = format!(
            //         "Memory (total): {} kb ({})",
            //         information.memory_total, memory_readable
            //     );
            //
            //     self.system_info.memory_text = if let Some(memory_used) = information.memory_used {
            //         let memory_readable = ByteSize::kb(memory_used).to_string();
            //         format!("{} kb ({})", memory_used, memory_readable)
            //     } else {
            //         "None".to_owned()
            //     };
            //
            //     self.system_info.memory_used = format!("Memory (used): {}", memory_text);
            //
            //     self.system_info.graphics_adapter =
            //         format!("Graphics adapter: {}", information.graphics_adapter);
            //
            //     self.system_info.graphics_backend =
            //         format!("Graphics backend: {}", information.graphics_backend);
            //
            //     Command::none()
            // }
            (false, LauncherMsg::ShowHelp) => Command::none(),
            (false, LauncherMsg::StartTask(i)) => self.run_task(self.task_paths[i].clone()),
            (true, LauncherMsg::TaskClose) => {
                self.busy = false;
                Command::none()
            }
            (true, LauncherMsg::TaskCrash(status)) => {
                self.busy = false;
                self.status_message = Some(status);
                Command::none()
            }
            (_, LauncherMsg::ToClipboard) => {
                if let Some(status) = self.status_message.as_ref() {
                    iced::clipboard::write(format!(
                        "Task \"{}\" failed with error:\n{status}",
                        self.active_task.as_ref().unwrap_or(&"[NONE]".to_owned())
                    ))
                } else {
                    iced::clipboard::write(format!("{:#?}", self.sys_info))
                }
            }
            (_, LauncherMsg::CloseCard) => {
                self.status_message = None;
                self.show_system_info = false;
                self.show_help = false;
                Command::none()
            }
            _ => Command::none(),
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let buttons: Vec<_> = self
            .task_labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let button = Button::new(
                    Text::new(label)
                        .size(TEXT_XSMALL)
                        .horizontal_alignment(Horizontal::Center)
                        .vertical_alignment(Vertical::Center),
                )
                .padding([15, 20])
                .width(Length::Units(450))
                .style(style::Select);

                if self.busy {
                    button
                } else {
                    button.on_press(LauncherMsg::StartTask(i))
                }
            })
            .collect();

        let content = if buttons.is_empty() {
            Container::new(Text::new("(No tasks found in task directory)").size(TEXT_XSMALL))
        } else {
            Container::new(Scrollable::new(style::grid(buttons, 1, 25, 25)))
        }
        .width(Length::Fill)
        .center_x()
        .center_y();

        let content = Container::new(
            Column::new()
                .spacing(25)
                .align_items(Alignment::Center)
                .push(
                    Container::new(
                        Column::new()
                            .spacing(3)
                            .align_items(Alignment::Center)
                            .push(
                                Text::new(if self.busy {
                                    format!("CogTask v{VERSION} (busy)")
                                } else {
                                    format!("CogTask v{VERSION}")
                                })
                                .size(TEXT_LARGE),
                            )
                            .push(
                                Row::new()
                                    .spacing(2)
                                    .align_items(Alignment::Center)
                                    .push(tooltip(
                                        button(Icon::Folder)
                                            .style(style::Transparent)
                                            .on_press(LauncherMsg::LoadTask),
                                        "Load task",
                                        Position::Bottom,
                                    ))
                                    .push(tooltip(
                                        button(Icon::FolderTree)
                                            .style(style::Transparent)
                                            .on_press(LauncherMsg::LoadTaskRepo),
                                        "Load task catalogue",
                                        Position::Bottom,
                                    ))
                                    .push(tooltip(
                                        button(Icon::SystemInfo)
                                            .style(style::Transparent)
                                            .on_press(LauncherMsg::ShowSystemInfo),
                                        "System information",
                                        Position::Bottom,
                                    ))
                                    .push(tooltip(
                                        button(Icon::Help)
                                            .style(style::Transparent)
                                            .on_press(LauncherMsg::ShowHelp),
                                        "Help",
                                        Position::Bottom,
                                    )),
                            ),
                    )
                    .center_x()
                    .center_y(),
                )
                .push(content),
        )
        .padding([30, 50, 50, 50])
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y();

        let status = self.status_message.clone().unwrap_or_else(|| "".to_owned());
        if !status.is_empty() || self.show_system_info {
            Modal::new(true, content, move || {
                Container::new(
                    Row::new()
                        .push(Space::with_width(Length::FillPortion(1)))
                        .push(
                            Column::new()
                                .push(Space::with_height(Length::FillPortion(1)))
                                .push(
                                    if !status.is_empty() {
                                        self.view_status()
                                    } else {
                                        self.view_sys_info()
                                    }
                                    .foot(
                                        Container::new(
                                            Row::new()
                                                .spacing(3)
                                                .padding(5)
                                                .align_items(Alignment::Center)
                                                .push(
                                                    button(
                                                        Icon::Clipboard,
                                                        // svg::Svg::new(svg::Handle::from_memory(
                                                        //     ICON_TO_CLIPBOARD,
                                                        // ))
                                                        // .content_fit(Contain),
                                                    )
                                                    .style(style::Transparent)
                                                    .width(Length::Units(36))
                                                    .height(Length::Units(36))
                                                    .on_press(LauncherMsg::ToClipboard),
                                                )
                                                .push(
                                                    button(
                                                        Icon::Close, // svg::Svg::new(svg::Handle::from_memory(
                                                                     //     ICON_CLOSE_WINDOW,
                                                                     // ))
                                                                     // .content_fit(Contain),
                                                    )
                                                    .style(style::Transparent)
                                                    .width(Length::Units(36))
                                                    .height(Length::Units(36))
                                                    .on_press(LauncherMsg::CloseCard),
                                                ),
                                        )
                                        .width(Length::Fill)
                                        .align_x(Horizontal::Right),
                                    )
                                    .height(Length::FillPortion(14))
                                    .padding_head(5.0),
                                )
                                .push(Space::with_height(Length::FillPortion(1)))
                                .width(Length::FillPortion(14)),
                        )
                        .push(Space::with_width(Length::FillPortion(1))),
                )
                .height(Length::Fill)
                .width(Length::Fill)
                .center_x()
                .center_y()
                .into()
            })
            .on_esc(LauncherMsg::CloseCard)
            .into()
        } else {
            content.into()
        }
    }
}

impl Launcher {
    fn view_sys_info(&self) -> Card<LauncherMsg, Renderer> {
        Card::new(
            Text::new("System information:").size(TEXT_XSMALL),
            column()
                .spacing(10)
                .align_items(Alignment::Start)
                .push(
                    text(format!(
                        "System: {} - {} - {}",
                        self.sys_info.sys_name, self.sys_info.sys_kernel, self.sys_info.sys_version
                    ))
                    .size(TEXT_TINY),
                )
                .push(
                    text(format!(
                        "CPU: {} - {}",
                        self.sys_info.cpu_brand, self.sys_info.cpu_cores
                    ))
                    .size(TEXT_TINY),
                )
                .push(
                    text(format!(
                        "Memory: {} - {}",
                        self.sys_info.memory_total, self.sys_info.memory_used
                    ))
                    .size(TEXT_TINY),
                )
                .push(
                    text(format!(
                        "Graphics: {} - {}",
                        self.sys_info.graphics_adapter, self.sys_info.graphics_backend
                    ))
                    .size(TEXT_TINY),
                ),
        )
        .style(style::Status)
    }

    fn view_status(&self) -> Card<LauncherMsg, Renderer> {
        use regex::Regex;
        let re = Regex::new(r"^\[[a-zA-Z\d]+\]$").unwrap();

        let text = self
            .status_message
            .as_ref()
            .map_or("".to_owned(), |s| s.clone());
        let text = text.lines().into_iter().map(|line| {
            if re.is_match(line) {
                Text::new(line).size(TEXT_TINY).color(CUSTOM_RED)
            } else {
                Text::new(line).size(TEXT_TINY).color(Color::BLACK)
            }
        });

        let mut content = column().spacing(5);
        for line in text {
            content = content.push(line);
        }

        Card::new(
            Text::new(format!(
                "Task \"{}\" failed with error:",
                self.active_task.as_ref().unwrap_or(&"[INVALID]".to_owned())
            ))
            .size(TEXT_TINY),
            Scrollable::new(content),
        )
        .style(style::Error)
    }
}
