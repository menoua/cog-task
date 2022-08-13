use crate::assets::{ICON_CLOSE_WINDOW, ICON_MAGNIFYING_GLASS, ICON_TO_CLIPBOARD};
use crate::config::Config;
use crate::env::Env;
use crate::logger::LoggerMsg;
use crate::resource::ResourceMap;
use crate::scheduler::{Scheduler, SchedulerMsg, SPIN_DURATION, SPIN_STRATEGY};
use crate::task::block::Block;
use crate::task::Task;
use crate::util::{f32_with_precision, str_with_precision};
use crate::{error, style};
use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::Event::KeyPressed;
use iced::pure::widget::{Button, Column, Container, Row, Scrollable, Space, Text, TextInput};
use iced::pure::{button, Application, Element};
use iced::ContentFit::{Contain, ScaleDown};
use iced::{svg, window, Alignment, Command, Renderer};
use iced_aw::pure::{Card, Modal};
use iced_native::{Event, Length, Subscription};
use serde_json::Value;
use spin_sleep::SpinSleeper;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Page {
    Startup,
    Selection,
    Loading,
    Activity,
    CleanUp,
}

pub struct Server {
    env: Env,
    task: Task,
    subject: String,
    scale_factor: f32,
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
}

impl Server {
    pub fn new(path: PathBuf) -> Result<Self, error::Error> {
        let env = Env::new(path)?;
        let task = Task::new(env.task())?;
        let blocks = task
            .block_labels()
            .into_iter()
            .map(|label| (label, false))
            .collect();

        Ok(Self {
            env,
            task,
            subject: "".to_owned(),
            scale_factor: 1.0,
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
        })
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
}

#[derive(Debug, Clone)]
pub enum ServerMsg {
    SetSubject(String),
    SetScale(f32),
    GoToSelect,
    GoToStartup,
    StartBlock(usize),
    LoadResources(usize),
    LoadComplete,
    FinishBlock,
    InterruptBlock,
    CrashBlock(error::Error),
    DropScheduler,
    CleanUp(Result<(), error::Error>),
    Quit,
    Relay(SchedulerMsg),
    ToClipboard,
    ClearStatus,
    Refresh(u32),
    ToggleSettings,
}

impl Application for Server {
    type Executor = iced::executor::Default;
    type Message = ServerMsg;
    type Flags = PathBuf;

    #[inline(always)]
    fn new(task: PathBuf) -> (Self, Command<Self::Message>) {
        match Server::new(task) {
            Ok(server) => (server, Command::none()),
            Err(e) => {
                eprintln!("[{}]\n{e:?}", e.type_());
                std::process::exit(1);
            }
        }
    }

    #[inline(always)]
    fn title(&self) -> String {
        format!("CogTask Server -- {}", self.task.title())
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match (self.page, message) {
            (Page::Startup, ServerMsg::SetSubject(subject)) => {
                self.subject = subject;
                Command::none()
            }
            (Page::Startup, ServerMsg::ToggleSettings) => {
                self.show_magnification = !self.show_magnification;
                Command::none()
            }
            (Page::Startup, ServerMsg::SetScale(scale)) => {
                self.scale_factor = scale;
                Command::none()
            }
            (Page::Startup, ServerMsg::GoToSelect) | (Page::Activity, ServerMsg::GoToSelect) => {
                self.page = Page::Selection;
                println!(
                    "Starting task with configuration:\n{:#?}",
                    self.task.config()
                );
                Command::none()
            }
            (Page::Selection, ServerMsg::GoToStartup) => {
                self.page = Page::Startup;
                Command::none()
            }
            (Page::Selection, ServerMsg::StartBlock(i)) => {
                if self.scheduler.is_none() {
                    println!("Starting experiment block {i}");
                    self.active_block = Some(i);
                    self.page = Page::Loading;
                    Command::perform(
                        async {
                            thread::sleep(Duration::from_millis(500));
                        },
                        move |()| ServerMsg::LoadResources(i),
                    )
                } else {
                    Command::none()
                }
            }
            (Page::Loading, ServerMsg::LoadResources(i)) => {
                if self.scheduler.is_none() {
                    let block = self.task.block(i);

                    if let Err(e) =
                        self.resources
                            .preload_block(block, &self.env, self.task.config())
                    {
                        return Command::perform(async move { e }, ServerMsg::CrashBlock);
                    }

                    match Scheduler::new(self) {
                        Ok((scheduler, cmd)) => {
                            self.scheduler = Some(scheduler);
                            Command::batch([
                                cmd,
                                Command::perform(async {}, |()| ServerMsg::LoadComplete),
                            ])
                        }
                        Err(e) => Command::perform(async move { e }, ServerMsg::CrashBlock),
                    }
                } else {
                    Command::none()
                }
            }
            (Page::Loading, ServerMsg::LoadComplete) => {
                match self.scheduler.as_mut().unwrap().start() {
                    Ok(cmd) => {
                        self.page = Page::Activity;
                        self.capture_key = true;
                        Command::batch([
                            cmd,
                            Command::perform(async {}, |()| SchedulerMsg::Refresh(0).wrap()),
                        ])
                    }
                    Err(e) => Command::perform(async move { e }, ServerMsg::CrashBlock),
                }
            }
            (Page::Activity, ServerMsg::FinishBlock) => {
                self.status = Some(Ok("Success".to_owned()));
                self.page = Page::CleanUp;
                self.capture_key = false;
                self.capture_fps = None;
                self.animation_id = 0;
                Command::perform(
                    async move {
                        thread::sleep(Duration::from_millis(500));
                    },
                    |()| ServerMsg::DropScheduler,
                )
            }
            (Page::Activity, ServerMsg::InterruptBlock) => {
                self.status = Some(Ok("Interrupted".to_owned()));
                self.page = Page::CleanUp;
                self.capture_key = false;
                self.capture_fps = None;
                self.animation_id = 0;
                Command::perform(
                    async move {
                        thread::sleep(Duration::from_millis(500));
                    },
                    |()| ServerMsg::DropScheduler,
                )
            }
            (Page::Loading | Page::Activity, ServerMsg::CrashBlock(e)) => {
                self.status = Some(Err(e.clone()));
                self.page = Page::CleanUp;
                self.capture_key = false;
                self.capture_fps = None;
                self.animation_id = 0;

                if let Some(scheduler) = &mut self.scheduler {
                    let _ = scheduler.update(SchedulerMsg::Logger(LoggerMsg::Append(
                        "mainevent".to_owned(),
                        ("crash".to_owned(), Value::String(format!("{e:#?}"))),
                    )));
                }

                Command::perform(
                    async move {
                        thread::sleep(Duration::from_millis(500));
                    },
                    |()| ServerMsg::DropScheduler,
                )
            }
            (Page::Loading | Page::Activity, ServerMsg::Relay(msg)) => {
                if let Some(scheduler) = self.scheduler.as_mut() {
                    match scheduler.update(msg) {
                        Ok(cmd) => {
                            if scheduler.captures_fps().is_none() {
                                self.capture_fps = None;
                                return cmd;
                            }

                            let animation_id = scheduler.animation_id();
                            if animation_id != self.animation_id {
                                let fps = scheduler.captures_fps();
                                self.animation_id = animation_id;
                                self.capture_fps = fps;
                                let sleeper = SpinSleeper::new(SPIN_DURATION)
                                    .with_spin_strategy(SPIN_STRATEGY);
                                let period = Duration::from_secs_f64(1.0 / fps.unwrap());
                                Command::batch([
                                    cmd,
                                    Command::perform(
                                        async move { sleeper.sleep(period) },
                                        move |()| ServerMsg::Refresh(animation_id),
                                    ),
                                ])
                            } else {
                                cmd
                            }
                        }
                        Err(e) => Command::perform(async move { e }, ServerMsg::CrashBlock),
                    }
                } else {
                    #[cfg(debug_assertions)]
                    println!("WW: Tried to send message to non-existent scheduler");
                    Command::none()
                }
            }
            (Page::Activity, ServerMsg::Refresh(i)) => {
                if i != self.animation_id {
                    Command::none()
                } else if let Some(scheduler) = self.scheduler.as_mut() {
                    if let Some(fps) = self.capture_fps {
                        let next_frame = Instant::now() + Duration::from_secs_f64(1.0 / fps);
                        match scheduler.update(SchedulerMsg::Refresh(i)) {
                            Ok(cmd) => {
                                let sleeper = SpinSleeper::new(SPIN_DURATION)
                                    .with_spin_strategy(SPIN_STRATEGY);
                                Command::batch([
                                    cmd,
                                    Command::perform(
                                        async move { sleeper.sleep(next_frame - Instant::now()) },
                                        move |()| ServerMsg::Refresh(i),
                                    ),
                                ])
                            }
                            Err(e) => Command::perform(async move { e }, ServerMsg::CrashBlock),
                        }
                    } else {
                        Command::none()
                    }
                } else {
                    Command::none()
                }
            }
            (Page::CleanUp, ServerMsg::DropScheduler) => {
                if let Some(mut scheduler) = self.scheduler.take() {
                    match scheduler.stop() {
                        Ok(cmd) => cmd,
                        Err(e) => {
                            Command::perform(async move { e }, |e| ServerMsg::CleanUp(Err(e)))
                        }
                    }
                } else {
                    Command::perform(async {}, |()| ServerMsg::CleanUp(Ok(())))
                }
            }
            (Page::CleanUp, ServerMsg::CleanUp(success)) => {
                match (&self.status, success) {
                    (Some(Ok(status)), Ok(_)) if status.as_str() == "Success" => {
                        self.blocks.get_mut(self.active_block.unwrap()).unwrap().1 = true;
                    }
                    (Some(Ok(status)), Err(e)) if status.as_str() == "Success" => {
                        self.status = Some(Err(e));
                    }
                    _ => {}
                }

                self.page = Page::Selection;
                Command::perform(async {}, |()| SchedulerMsg::Refresh(0).wrap())
            }
            (Page::Selection, ServerMsg::ToClipboard) => {
                if let Some(status) = self.status.as_ref() {
                    match status {
                        Ok(s) => iced::clipboard::write(format!(
                            "Block \"{}\" ended with status:\n{s}",
                            self.active_block.map_or("", |i| &self.blocks[i].0)
                        )),
                        Err(e) => iced::clipboard::write(format!(
                            "Block \"{}\" failed with {}:\n{e:?}",
                            self.active_block.map_or("", |i| &self.blocks[i].0),
                            e.type_()
                        )),
                    }
                } else {
                    Command::none()
                }
            }
            (Page::Selection, ServerMsg::ClearStatus) => {
                self.active_block = None;
                self.status = None;
                Command::none()
            }
            (Page::Startup, ServerMsg::Quit) => std::process::exit(0),
            _ => Command::none(),
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        if self.capture_key {
            iced_native::subscription::events_with(|event, _status| match event {
                Event::Keyboard(KeyPressed { key_code, .. }) => {
                    Some(SchedulerMsg::KeyPress(key_code).wrap())
                }
                _ => None,
            })
        } else {
            Subscription::none()
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        match self.page {
            Page::Startup => self.view_startup(),
            Page::Selection => self.view_selection(),
            Page::Activity => self.view_activity(),
            Page::Loading => self.view_loading(),
            Page::CleanUp => self.view_cleanup(),
        }
    }

    #[inline(always)]
    fn mode(&self) -> window::Mode {
        window::Mode::Fullscreen
    }

    #[inline(always)]
    fn scale_factor(&self) -> f64 {
        self.scale_factor as f64
    }
}

impl Server {
    fn view_startup(&self) -> Element<'_, <Self as Application>::Message> {
        let controls = Row::new()
            .align_items(Alignment::Center)
            .spacing(75)
            .push(
                Button::new(
                    Text::new("Quit")
                        .size(40)
                        .horizontal_alignment(Horizontal::Center)
                        .vertical_alignment(Vertical::Center),
                )
                .padding([15, 60])
                .style(style::Cancel)
                .on_press(ServerMsg::Quit),
            )
            .push({
                let row = Row::new()
                    .align_items(Alignment::Center)
                    .spacing(5)
                    .height(Length::Shrink)
                    .push(
                        Button::new(
                            svg::Svg::new(svg::Handle::from_memory(ICON_MAGNIFYING_GLASS))
                                .width(Length::Units(30))
                                .content_fit(ScaleDown),
                        )
                        .style(style::Transparent)
                        .on_press(ServerMsg::ToggleSettings),
                    );

                if self.show_magnification {
                    row.push(
                        Text::new(format!("| x{} ", str_with_precision(self.scale_factor, 1)))
                            .size(30)
                            .horizontal_alignment(Horizontal::Center),
                    )
                    .push({
                        Button::new(
                            Text::new("A")
                                .size(18)
                                .horizontal_alignment(Horizontal::Center)
                                .vertical_alignment(Vertical::Center),
                        )
                        .on_press({
                            let scale = f32_with_precision(self.scale_factor - 0.2, 1).max(0.8);
                            ServerMsg::SetScale(scale)
                        })
                        .width(Length::Units(30))
                        .height(Length::Units(30))
                        .style(style::Select)
                    })
                    .push({
                        Button::new(
                            Text::new("A")
                                .size(28)
                                .horizontal_alignment(Horizontal::Center)
                                .vertical_alignment(Vertical::Center),
                        )
                        .on_press({
                            let scale = f32_with_precision(self.scale_factor + 0.2, 1).min(1.2);
                            ServerMsg::SetScale(scale)
                        })
                        .width(Length::Units(30))
                        .height(Length::Units(30))
                        .style(style::Select)
                    })
                } else {
                    row
                }
            })
            .push(
                Row::new()
                    .spacing(15)
                    .align_items(Alignment::Center)
                    .push(
                        Text::new("Subject ID: ")
                            .size(36)
                            .horizontal_alignment(Horizontal::Center)
                            .vertical_alignment(Vertical::Center),
                    )
                    .push(
                        TextInput::new("Enter Subject ID", &self.subject, ServerMsg::SetSubject)
                            .size(36)
                            .width(Length::Units(300))
                            .padding([5, 9]),
                    ),
            )
            .push({
                let button = Button::new(
                    Text::new("Start!")
                        .size(40)
                        .horizontal_alignment(Horizontal::Center)
                        .vertical_alignment(Vertical::Center),
                )
                .padding([15, 60])
                .style(style::Submit);

                if self.subject.is_empty()
                    | !self
                        .subject
                        .chars()
                        .all(|c| c.is_alphabetic() || c.is_alphanumeric() | "-_".contains(c))
                {
                    button
                } else {
                    button.on_press(ServerMsg::GoToSelect)
                }
            });

        Container::new(
            Column::new()
                .width(Length::Fill)
                .align_items(Alignment::Center)
                .spacing(75)
                .push(
                    Container::new(
                        Text::new(self.task.title())
                            .size(45)
                            .horizontal_alignment(Horizontal::Center),
                    )
                    .align_y(Vertical::Top),
                )
                .push(
                    Container::new(Scrollable::new(
                        Text::new(self.task.description())
                            .width(Length::Units(1200))
                            .size(36)
                            .horizontal_alignment(Horizontal::Center),
                    ))
                    .height(Length::Fill)
                    .center_y(),
                )
                .push(Container::new(controls.height(Length::Units(75))).align_y(Vertical::Bottom)),
        )
        .padding(75)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into()
    }

    fn view_selection(&self) -> Element<'_, <Self as Application>::Message> {
        let content = Column::new()
            .spacing(45)
            .align_items(Alignment::Center)
            .padding([100, 0, 0, 0])
            .push(
                Container::new(Scrollable::new(style::grid(
                    self.blocks
                        .iter()
                        .enumerate()
                        .map(|(i, (block, done))| {
                            let button = Button::new(Text::new(block).size(36))
                                .padding([15, 60])
                                .on_press(ServerMsg::StartBlock(i));

                            if *done {
                                button.style(style::Done)
                            } else {
                                button.style(style::Select)
                            }
                        })
                        .collect(),
                    self.task.config().blocks_per_row() as usize,
                    40,
                    40,
                )))
                .height(Length::Fill)
                .center_y(),
            )
            .push(
                Container::new(
                    Button::new(Text::new("Back").size(30))
                        .padding([15, 60])
                        .style(style::Cancel)
                        .on_press(ServerMsg::GoToStartup),
                )
                .height(Length::Units(60))
                .align_y(Vertical::Bottom),
            );

        let content = Container::new(content)
            .padding(75)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y();

        match &self.status {
            None => content.into(),
            Some(status) => Modal::new(true, content, move || {
                Container::new(
                    Column::new()
                        .push(Space::with_height(Length::FillPortion(1)))
                        .push(self.view_status(status).height(Length::FillPortion(14)))
                        .push(Space::with_height(Length::FillPortion(1)))
                        .width(Length::Units(if status.is_err() { 1200 } else { 900 })),
                )
                .height(Length::Fill)
                .width(Length::Fill)
                .center_x()
                .center_y()
                .into()
            })
            .on_esc(ServerMsg::ClearStatus)
            .into(),
        }
    }

    fn view_activity(&self) -> Element<'_, <Self as Application>::Message> {
        if let Some(scheduler) = self.scheduler.as_ref() {
            match scheduler.view(self.scale_factor) {
                Ok(view) => Column::new()
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_items(Alignment::Center)
                    .push(view)
                    .into(),
                Err(e) => {
                    #[cfg(debug_assertions)]
                    println!("View error: {e:#?}");
                    panic!("Error encountered during view call:\n{e:#?}");
                }
            }
        } else {
            Column::new().into()
        }
    }

    fn view_loading(&self) -> Element<'_, <Self as Application>::Message> {
        Container::new(Text::new("...").size(45))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn view_cleanup(&self) -> Element<'_, <Self as Application>::Message> {
        Container::new(Text::new("...").size(45))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

impl Server {
    fn view_status(&self, status: &Result<String, error::Error>) -> Card<ServerMsg, Renderer> {
        match status {
            Ok(s) => self.view_status_ok(s),
            Err(e) => self.view_status_err(e),
        }
        .padding_head(7.5)
        .foot(
            Container::new(
                Row::new()
                    .spacing(5)
                    .padding(8)
                    .align_items(Alignment::Center)
                    .push(
                        button(
                            svg::Svg::new(svg::Handle::from_memory(ICON_TO_CLIPBOARD))
                                .content_fit(Contain),
                        )
                        .style(style::Transparent)
                        .width(Length::Units(50))
                        .height(Length::Units(50))
                        .on_press(ServerMsg::ToClipboard),
                    )
                    .push(
                        button(
                            svg::Svg::new(svg::Handle::from_memory(ICON_CLOSE_WINDOW))
                                .content_fit(Contain),
                        )
                        .style(style::Transparent)
                        .width(Length::Units(50))
                        .height(Length::Units(50))
                        .on_press(ServerMsg::ClearStatus),
                    ),
            )
            .width(Length::Fill)
            .align_x(Horizontal::Right),
        )
    }

    fn view_status_ok(&self, status: &String) -> Card<ServerMsg, Renderer> {
        let message = Card::new(
            Text::new(format!(
                "Block \"{}\" ended with status:",
                self.active_block.map_or("", |i| &self.blocks[i].0)
            ))
            .size(36),
            Scrollable::new(Text::new(status).size(36)),
        );

        if status.as_str() == "Success" {
            message.style(style::Success)
        } else {
            message.style(style::Status)
        }
    }

    fn view_status_err(&self, err: &error::Error) -> Card<ServerMsg, Renderer> {
        Card::new(
            Text::new(format!(
                "Block \"{}\" failed with {}:",
                self.active_block.map_or("", |i| &self.blocks[i].0),
                err.type_()
            ))
            .size(36),
            Scrollable::new(Text::new(format!("{err:?}")).size(36)),
        )
        .style(style::Error)
    }
}
