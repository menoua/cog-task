use crate::action::{Action, StatefulAction};
use crate::assets::{SPIN_DURATION, SPIN_STRATEGY};
use crate::config::Config;
use crate::error;
use crate::error::Error::{InternalError, InvalidResourceError, TaskDefinitionError};
use crate::io::IO;
use crate::resource::{ResourceMap, ResourceValue};
use crate::scheduler::{monitor::Monitor, SchedulerMsg};
use crate::server::ServerMsg;
use iced::pure::widget::{image, Container};
use iced::pure::Element;
use iced::{Command, ContentFit, Length};
use serde::{Deserialize, Serialize};
use spin_sleep::SpinSleeper;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
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
    volume: Option<f32>,
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
    fn init(&self) -> Result<(), error::Error> {
        match self.volume {
            Some(f) if !(0.0..=1.0).contains(&f) => Err(TaskDefinitionError(
                "Stream volume should be a float number between 0.0 and 1.0".to_owned(),
            )),
            _ => Ok(()),
        }
    }

    fn stateful(
        &self,
        id: usize,
        res: &ResourceMap,
        config: &Config,
        _io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        match res.fetch(&self.src())? {
            ResourceValue::Stream(stream) => {
                let volume = config.volume(self.volume);
                let mut stream = stream.cloned(volume)?;

                let frame = Arc::new(Mutex::new(None));
                if stream.has_video() {
                    stream.set_callbacks(frame.clone())?;
                } else if self.width.is_some() {
                    return Err(TaskDefinitionError(format!(
                        "Video-less stream `{id}` should not be supplied a width"
                    )));
                }

                let framerate = stream.framerate();
                let sleeper = SpinSleeper::new(SPIN_DURATION).with_spin_strategy(SPIN_STRATEGY);
                let period = if stream.has_video() {
                    Duration::from_secs_f64(0.5 / framerate)
                } else {
                    Duration::from_millis(5)
                };

                let done = Arc::new(Mutex::new(Ok(stream.eos())));
                let (tx_start, rx_start) = mpsc::channel();
                let (tx_stop, rx_stop) = mpsc::channel();
                let looping = self.looping;

                let done_clone = done.clone();
                let join_handle = thread::spawn(move || -> Result<(), error::Error> {
                    if rx_start.recv().is_err() {
                        return Ok(());
                    }

                    stream.start()?;

                    loop {
                        if let Err(TryRecvError::Disconnected) = rx_start.try_recv() {
                            stream.pause()?;
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

                    let _ = tx_stop.send(());
                    Ok(())
                });

                Ok(Box::new(StatefulStream {
                    id,
                    done,
                    frame,
                    framerate,
                    width: self.width,
                    looping,
                    link: Some((tx_start, rx_stop)),
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
    link: Option<(Sender<()>, Receiver<()>)>,
    join_handle: Option<JoinHandle<Result<(), error::Error>>>,
}

impl StatefulAction for StatefulStream {
    #[inline(always)]
    fn id(&self) -> usize {
        self.id
    }

    #[inline(always)]
    fn is_over(&self) -> Result<bool, error::Error> {
        self.done.lock().unwrap().clone()
    }

    #[inline(always)]
    fn is_visual(&self) -> bool {
        self.framerate > 0.0
    }

    #[inline(always)]
    fn is_static(&self) -> bool {
        self.looping
    }

    #[inline(always)]
    fn monitors(&self) -> Option<Monitor> {
        if self.framerate > 0.0 {
            Some(Monitor::Frames(self.framerate))
        } else {
            None
        }
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        *self.done.lock().unwrap() = Ok(true);
        Ok(())
    }

    fn start(&mut self) -> Result<Command<ServerMsg>, error::Error> {
        let link = self.link.take().ok_or_else(|| {
            InternalError(format!(
                "Link to streaming thread could not be acquired for action `{}`",
                self.id
            ))
        })?;

        link.0.send(()).map_err(|e| {
            InternalError(format!(
                "Failed to send start signal to concurrent stream thread:\n{e:#?}"
            ))
        })?;

        let done = self.done.clone();
        let join_handle = self.join_handle.take().ok_or_else(|| {
            InternalError(format!(
                "JoinHandle for action `{}` has died prematurely",
                self.id,
            ))
        })?;

        Ok(Command::batch([
            Command::perform(async {}, |()| SchedulerMsg::Refresh(0).wrap()),
            Command::perform(
                async move {
                    let link = link;
                    let _ = link.1.recv();
                    *done.lock().unwrap() = match join_handle.join() {
                        Ok(Ok(_)) => Ok(true),
                        Ok(Err(e)) => Err(e),
                        Err(e) => Err(InternalError(format!(
                            "Failed to graciously close stream decoder thread:\n{e:#?}"
                        ))),
                    };
                },
                |()| SchedulerMsg::Advance.wrap(),
            ),
        ]))
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
