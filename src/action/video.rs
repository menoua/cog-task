use crate::action::{Action, StatefulAction, StatefulActionMsg};
use crate::config::Config;
use crate::error;
use crate::error::Error::{InternalError, InvalidResourceError};
use crate::io::IO;
use crate::resource::{ResourceMap, ResourceValue};
use crate::scheduler::{Event, Monitor, SchedulerMsg, SPIN_DURATION, SPIN_STRATEGY};
use crate::server::ServerMsg;
use iced::pure::widget::{image, Container};
use iced::pure::Element;
use iced::{ContentFit, Length};
use iced_native::Command;
use serde::{Deserialize, Serialize};
use spin_sleep::SpinSleeper;
use std::path::PathBuf;
use std::sync::mpsc::{Sender, TryRecvError};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Video {
    src: PathBuf,
    #[serde(default)]
    width: Option<u16>,
    #[serde(default)]
    looping: bool,
    #[serde(default = "defaults::cache")]
    cache: bool,
    #[serde(default)]
    style: String,
}

mod defaults {
    #[inline(always)]
    pub fn cache() -> bool {
        false
    }
}

impl Video {
    #[inline(always)]
    fn src(&self) -> PathBuf {
        if self.cache {
            PathBuf::from(format!("{}#cache", self.src.to_str().unwrap()))
        } else {
            PathBuf::from(format!("{}#stream", self.src.to_str().unwrap()))
        }
    }
}

impl Action for Video {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![self.src()]
    }

    fn stateful(
        &self,
        id: usize,
        res: &ResourceMap,
        _config: &Config,
        _io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        match res.fetch(&self.src())? {
            ResourceValue::Video(frames, framerate) => Ok(Box::new(StatefulVideo {
                id,
                done: Arc::new(Mutex::new(frames.is_empty())),
                frames: frames.clone(),
                framerate: *framerate,
                position: Arc::new(Mutex::new(0)),
                width: self.width,
                looping: self.looping,
            })),
            ResourceValue::Stream(video) => {
                let frame = Arc::new(Mutex::new(None));
                let mut video = video.cloned()?;
                video.set_sink_callback(frame.clone())?;
                let framerate = video.framerate();
                let done = Arc::new(Mutex::new(Ok(video.eos())));
                let sleeper = SpinSleeper::new(SPIN_DURATION).with_spin_strategy(SPIN_STRATEGY);
                let period = Duration::from_secs_f64(0.5 / framerate);
                let (tx, rx) = mpsc::channel();
                let looping = self.looping;

                let done_clone = done.clone();
                let join_handle = thread::spawn(move || -> Result<(), error::Error> {
                    if rx.recv().is_err() {
                        return Ok(());
                    }

                    video.start_stream()?;

                    loop {
                        if let Err(TryRecvError::Disconnected) = rx.try_recv() {
                            video.set_paused(true)?;
                            break;
                        }

                        sleeper.sleep(period);
                        let mut done = done_clone.lock().unwrap();
                        match (video.eos(), video.process_bus(looping)) {
                            (true, _) => *done = Ok(true),
                            (false, Ok(true)) => *done = Ok(true),
                            (false, Err(e)) => *done = Err(e),
                            _ => {}
                        }
                        if let Ok(true) = *done {
                            break;
                        }
                    }

                    Ok(())
                });

                Ok(Box::new(StatefulStream {
                    id,
                    done,
                    frame,
                    framerate,
                    width: self.width,
                    looping,
                    link: tx,
                    join_handle: Some(join_handle),
                }))
            }
            _ => Err(InvalidResourceError(format!(
                "Video action supplied non-video resource: `{:?}`",
                self.src
            ))),
        }
    }
}

#[derive(Debug)]
pub struct StatefulVideo {
    id: usize,
    done: Arc<Mutex<bool>>,
    frames: Arc<Vec<image::Handle>>,
    framerate: f64,
    position: Arc<Mutex<usize>>,
    width: Option<u16>,
    looping: bool,
}

impl StatefulAction for StatefulVideo {
    #[inline(always)]
    fn id(&self) -> usize {
        self.id
    }

    #[inline(always)]
    fn is_over(&self) -> bool {
        *self.done.lock().unwrap()
    }

    #[inline(always)]
    fn is_visual(&self) -> bool {
        true
    }

    #[inline(always)]
    fn is_static(&self) -> bool {
        self.looping
    }

    #[inline(always)]
    fn monitors(&self) -> Option<Monitor> {
        Some(Monitor::Frames(self.framerate))
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        *self.done.lock().unwrap() = true;
        Ok(())
    }

    #[inline(always)]
    fn start(&mut self) -> Result<Command<ServerMsg>, error::Error> {
        *self.position.lock().unwrap() = 0;

        let sleeper = SpinSleeper::new(SPIN_DURATION).with_spin_strategy(SPIN_STRATEGY);
        let period = Duration::from_secs_f64(1.0 / self.framerate);
        let position = self.position.clone();
        let done = self.done.clone();
        let n_frames = self.frames.len();
        let looping = self.looping;

        thread::spawn(move || loop {
            sleeper.sleep(period);
            let mut done = done.lock().unwrap();
            let mut pos = position.lock().unwrap();
            if *pos == n_frames - 1 {
                if looping {
                    *pos = 0;
                } else {
                    *done = true;
                }
            } else {
                *pos += 1;
            }
            if *done {
                break;
            }
        });

        Ok(Command::none())
    }

    fn update(&mut self, msg: StatefulActionMsg) -> Result<Command<ServerMsg>, error::Error> {
        if let StatefulActionMsg::UpdateEvent(Event::Refresh) = msg {
            if *self.done.lock().unwrap() {
                Ok(Command::perform(async {}, |()| {
                    SchedulerMsg::Advance.wrap()
                }))
            } else {
                Ok(Command::none())
            }
        } else {
            Ok(Command::none())
        }
    }

    fn view(&self, scale_factor: f32) -> Result<Element<'_, ServerMsg>, error::Error> {
        let position = *self.position.lock().unwrap();
        let image =
            image::Image::new(self.frames[position].clone()).content_fit(ContentFit::ScaleDown);

        Ok(if let Some(width) = self.width {
            let width = (scale_factor * width as f32) as u16;
            Container::new(
                Container::new(
                    image
                        .content_fit(ContentFit::Contain)
                        .width(Length::Units(width))
                        .height(Length::Fill),
                )
                .width(Length::Units(width))
                .height(Length::Fill)
                .center_x()
                .center_y(),
            )
        } else {
            Container::new(image.content_fit(ContentFit::ScaleDown))
                .height(Length::Fill)
                .center_x()
                .center_y()
        }
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into())
    }
}

#[derive(Debug)]
pub struct StatefulStream {
    id: usize,
    done: Arc<Mutex<Result<bool, error::Error>>>,
    // video: Arc<Mutex<Option<video::Handle>>>,
    frame: Arc<Mutex<Option<image::Handle>>>,
    framerate: f64,
    width: Option<u16>,
    looping: bool,
    link: Sender<()>,
    join_handle: Option<JoinHandle<Result<(), error::Error>>>,
}

impl StatefulAction for StatefulStream {
    #[inline(always)]
    fn id(&self) -> usize {
        self.id
    }

    #[inline(always)]
    fn is_over(&self) -> bool {
        *(*self.done.lock().unwrap()).as_ref().unwrap_or(&true)
    }

    #[inline(always)]
    fn is_visual(&self) -> bool {
        true
    }

    #[inline(always)]
    fn is_static(&self) -> bool {
        self.looping
    }

    #[inline(always)]
    fn monitors(&self) -> Option<Monitor> {
        Some(Monitor::Frames(self.framerate))
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        *self.done.lock().unwrap() = Ok(true);
        Ok(())
    }

    fn start(&mut self) -> Result<Command<ServerMsg>, error::Error> {
        self.link.send(()).map_err(|e| {
            InternalError(format!(
                "Failed to send start signal to concurrent video thread:\n{e:#?}"
            ))
        })?;

        Ok(Command::perform(
            async {
                thread::sleep(Duration::from_millis(10));
            },
            |()| SchedulerMsg::Refresh(0).wrap(),
        ))
    }

    fn update(&mut self, msg: StatefulActionMsg) -> Result<Command<ServerMsg>, error::Error> {
        if let StatefulActionMsg::UpdateEvent(Event::Refresh) = msg {
            let thread_died = match self.join_handle.as_ref() {
                None => Err(InternalError(
                    "Failed to graciously close video decoder thread:\nJoinHandle is missing"
                        .to_owned(),
                ))?,
                Some(join_handle) => join_handle.is_finished(),
            };
            if thread_died {
                match self.join_handle.take().unwrap().join() {
                    Ok(Ok(_)) => *self.done.lock().unwrap() = Ok(true),
                    Ok(Err(e)) => Err(e)?,
                    Err(e) => Err(InternalError(format!(
                        "Failed to graciously close video decoder thread:\n{e:#?}"
                    )))?,
                }

                Ok(Command::perform(async {}, |()| {
                    SchedulerMsg::Advance.wrap()
                }))
            } else {
                match self.done.lock().unwrap().as_ref() {
                    Ok(true) => Ok(Command::perform(async {}, |()| {
                        SchedulerMsg::Advance.wrap()
                    })),
                    Ok(false) => Ok(Command::none()),
                    Err(e) => Err(e.clone()),
                }
            }
        } else {
            Ok(Command::none())
        }
    }

    fn view(&self, scale_factor: f32) -> Result<Element<'_, ServerMsg>, error::Error> {
        let image = image::Image::new(
            self.frame
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| image::Handle::from_pixels(0, 0, vec![])),
        )
        .content_fit(ContentFit::ScaleDown);

        Ok(if let Some(width) = self.width {
            let width = (scale_factor * width as f32) as u16;
            Container::new(
                Container::new(
                    image
                        .content_fit(ContentFit::Contain)
                        .width(Length::Units(width))
                        .height(Length::Fill),
                )
                .width(Length::Units(width))
                .height(Length::Fill)
                .center_x()
                .center_y(),
            )
        } else {
            Container::new(image.content_fit(ContentFit::ScaleDown))
                .height(Length::Fill)
                .center_x()
                .center_y()
        }
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into())
    }
}

impl Drop for StatefulStream {
    fn drop(&mut self) {
        self.stop().unwrap_or_else(|e| {
            eprintln!("{e:#?}");
            std::process::exit(2);
        });
    }
}
