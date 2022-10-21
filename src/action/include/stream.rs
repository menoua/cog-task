use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE, VISUAL};
use crate::backend::MediaMode;
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::error::Error::{InternalError, InvalidResourceError, TaskDefinitionError};
use crate::io::IO;
use crate::resource::audio::Trigger;
use crate::resource::color::Color;
use crate::resource::{ResourceMap, ResourceValue};
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use crate::util::spin_sleeper;
use eframe::egui;
use eframe::egui::{CentralPanel, CursorIcon, Frame, TextureId, Vec2};
use serde::{Deserialize, Serialize};
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
    trigger: Trigger,
    #[serde(default)]
    background: Color,
}

stateful_arc!(Stream {
    frame: Arc<Mutex<Option<(TextureId, Vec2)>>>,
    framerate: f64,
    width: Option<u16>,
    looping: bool,
    link_start: Sender<()>,
    link_stop: Option<Receiver<()>>,
    join_handle: Option<JoinHandle<Result<(), error::Error>>>,
    background: Color,
});

impl Stream {
    #[inline(always)]
    fn src(&self) -> PathBuf {
        PathBuf::from(format!("{}#stream", self.src.to_str().unwrap()))
    }
}

impl Action for Stream {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        if let Trigger::Ext(trig) = &self.trigger {
            vec![self.src(), trig.clone()]
        } else {
            vec![self.src()]
        }
    }

    #[inline]
    fn init(self) -> Result<Box<dyn Action>, error::Error> {
        if let Some(v) = self.volume {
            if v < 0.0 || v > 1.0 {
                return Err(TaskDefinitionError(
                    "Stream volume should be a float number between 0.0 and 1.0".to_owned(),
                ));
            }
        }

        Ok(Box::new(self))
    }

    fn stateful(
        &self,
        _io: &IO,
        res: &ResourceMap,
        config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        let stream = if let ResourceValue::Stream(stream) = res.fetch(&self.src())? {
            stream
        } else {
            return Err(InvalidResourceError(format!(
                "Stream action supplied non-stream resource: `{:?}`",
                self.src
            )));
        };

        let use_trigger = config.use_trigger();
        let media_mode = match (&self.trigger, use_trigger) {
            (Trigger::Ext(trig), true) => MediaMode::WithExtTrigger(trig.to_owned()),
            (Trigger::Int, false) => MediaMode::SansIntTrigger,
            _ => MediaMode::Normal,
        };

        let frame = Arc::new(Mutex::new(None));
        let volume = config.volume(self.volume);
        let mut stream = stream.cloned(frame.clone(), media_mode, volume)?;

        if !stream.has_video() && self.width.is_some() {
            return Err(TaskDefinitionError(format!(
                "Video-less stream `?` should not be supplied a width"
            )));
        }

        let framerate = stream.framerate();
        let sleeper = spin_sleeper();
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
            done,
            frame,
            framerate,
            width: self.width,
            looping,
            link_start: tx_start,
            link_stop: Some(rx_stop),
            join_handle: Some(join_handle),
            background: self.background,
        }))
    }
}

impl StatefulAction for StatefulStream {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        match (self.framerate, self.looping) {
            (f, false) if f > 0.0 => VISUAL,
            (f, true) if f > 0.0 => INFINITE | VISUAL,
            (_, false) => DEFAULT,
            (_, true) => INFINITE,
        }
        .into()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        self.link_start.send(()).map_err(|e| {
            InternalError(format!(
                "Failed to send start signal to concurrent stream thread:\n{e:#?}"
            ))
        })?;

        let rx_stop = self.link_stop.take().ok_or_else(|| {
            InternalError(format!(
                "Link to streaming thread could not be acquired for action"
            ))
        })?;

        let done = self.done.clone();
        let join_handle = self
            .join_handle
            .take()
            .ok_or_else(|| InternalError(format!("JoinHandle for action has died prematurely")))?;

        if let Ok(true) = *self.done.lock().unwrap() {
            sync_writer.push(SyncSignal::UpdateGraph);
            return Ok(());
        }

        {
            let mut sync_writer = sync_writer.clone();

            thread::spawn(move || {
                let _ = rx_stop.recv();
                *done.lock().unwrap() = match join_handle.join() {
                    Ok(Ok(_)) => Ok(true),
                    Ok(Err(e)) => Err(e),
                    Err(e) => Err(InternalError(format!(
                        "Failed to graciously close stream decoder thread:\n{e:#?}"
                    ))),
                };
                sync_writer.push(SyncSignal::UpdateGraph);
            });
        }

        sync_writer.push(SyncSignal::Repaint);
        Ok(())
    }

    fn update(
        &mut self,
        _signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        let (texture, size) = self
            .frame
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| (TextureId::default(), Vec2::splat(1.0)));

        ui.output().cursor_icon = CursorIcon::None;

        CentralPanel::default()
            .frame(Frame::default().fill(self.background.into()))
            .show_inside(ui, |ui| {
                ui.centered_and_justified(|ui| {
                    if let Some(width) = self.width {
                        let scale = width as f32 / size.x;
                        ui.image(texture, size * scale);
                    } else {
                        ui.image(texture, size);
                    }
                });
            });

        if self.framerate > 0.0 {
            ui.ctx().request_repaint();
        }
        Ok(())
    }

    #[inline(always)]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        *self.done.lock().unwrap() = Ok(true);
        if self.framerate > 0.0 {
            sync_writer.push(SyncSignal::Repaint);
        }
        Ok(())
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([
                ("framerate", format!("{:?}", self.framerate)),
                ("looping", format!("{:?}", self.looping)),
            ])
            .collect()
    }
}
