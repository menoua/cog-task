use crate::action::{Action, StatefulAction, StatefulActionMsg};
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::error::Error::{InternalError, InvalidResourceError, TaskDefinitionError};
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
pub struct Stream {
    src: PathBuf,
    #[serde(default)]
    width: Option<u16>,
    #[serde(default)]
    volume: Option<f64>,
    #[serde(default)]
    looping: bool,
    #[serde(default)]
    style: String,
}

impl Stream {
    #[inline(always)]
    fn src(&self) -> PathBuf {
        PathBuf::from(format!("{}#stream", self.src.to_str().unwrap()))
    }
}

impl Action for Stream {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![self.src()]
    }

    #[inline(always)]
    fn init(&self) -> Result<(), Error> {
        match self.volume {
            Some(f) if f < 0.0 || f > 1.0 => Err(TaskDefinitionError(
                "Stream volume should be a float number between 0.0 and 1.0".to_owned(),
            )),
            _ => Ok(()),
        }
    }

    fn stateful(
        &self,
        id: usize,
        res: &ResourceMap,
        _config: &Config,
        _io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        match res.fetch(&self.src())? {
            ResourceValue::Stream(stream) => {
                let frame = Arc::new(Mutex::new(None));
                let mut stream = stream.cloned()?;
                if let Some(volume) = self.volume {
                    stream.set_volume(volume);
                }
                stream.set_sink_callback(frame.clone())?;
                let framerate = stream.framerate();
                let done = Arc::new(Mutex::new(Ok(stream.eos())));
                let sleeper = SpinSleeper::new(SPIN_DURATION).with_spin_strategy(SPIN_STRATEGY);
                let period = Duration::from_secs_f64(0.5 / framerate);
                let (tx, rx) = mpsc::channel();
                let looping = self.looping;

                let done_clone = done.clone();
                let join_handle = thread::spawn(move || -> Result<(), error::Error> {
                    if rx.recv().is_err() {
                        return Ok(());
                    }

                    stream.start_stream()?;

                    loop {
                        if let Err(TryRecvError::Disconnected) = rx.try_recv() {
                            stream.set_paused(true)?;
                            break;
                        }

                        sleeper.sleep(period);
                        let mut done = done_clone.lock().unwrap();
                        match (stream.eos(), stream.process_bus(looping)) {
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
                "Stream action supplied non-stream resource: `{:?}`",
                self.src
            ))),
        }
    }
}

#[derive(Debug)]
pub struct StatefulStream {
    id: usize,
    done: Arc<Mutex<Result<bool, error::Error>>>,
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
                "Failed to send start signal to concurrent stream thread:\n{e:#?}"
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
                    "Failed to graciously close stream decoder thread:\nJoinHandle is missing"
                        .to_owned(),
                ))?,
                Some(join_handle) => join_handle.is_finished(),
            };
            if thread_died {
                match self.join_handle.take().unwrap().join() {
                    Ok(Ok(_)) => *self.done.lock().unwrap() = Ok(true),
                    Ok(Err(e)) => Err(e)?,
                    Err(e) => Err(InternalError(format!(
                        "Failed to graciously close stream decoder thread:\n{e:#?}"
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
