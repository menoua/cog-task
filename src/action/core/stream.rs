//@ stream

use crate::action::{Action, Props, StatefulAction, DEFAULT, INFINITE, VISUAL};
use crate::comm::{QWriter, Signal};
use crate::resource::{
    AudioChannel, Color, IoManager, OptionalFloat, ResourceAddr, ResourceManager, ResourceValue,
    StreamMode, Volume,
};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use crate::util::spin_sleeper;
use eframe::egui;
use eframe::egui::{CentralPanel, Color32, Frame, Response, TextureId, Vec2};
use eyre::{eyre, Context, Result};
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
    width: OptionalFloat,
    #[serde(default)]
    volume: Volume,
    #[serde(default)]
    channel: AudioChannel,
    #[serde(default)]
    looping: bool,
    #[serde(default)]
    background: Color,
    #[serde(default = "defaults::pad")]
    pad: bool,
}

stateful_arc!(Stream {
    frame: Arc<Mutex<Option<(TextureId, Vec2)>>>,
    framerate: f64,
    width: Option<f32>,
    looping: bool,
    link_start: Sender<()>,
    link_stop: Option<Receiver<()>>,
    join_handle: Option<JoinHandle<Result<()>>>,
    background: Color32,
    pad: bool,
});

mod defaults {
    pub fn pad() -> bool {
        true
    }
}

impl Action for Stream {
    #[inline]
    fn init(self) -> Result<Box<dyn Action>> {
        if let Volume::Value(vol) = self.volume {
            if !(0.0..=1.0).contains(&vol) {
                return Err(eyre!(
                    "Stream volume should be a float number between 0.0 and 1.0"
                ));
            }
        }

        Ok(Box::new(self))
    }

    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<ResourceAddr> {
        vec![ResourceAddr::Stream(self.src.clone())]
    }

    fn stateful(
        &self,
        _io: &IoManager,
        res: &ResourceManager,
        config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        let src = ResourceAddr::Stream(self.src.clone());
        let stream = if let ResourceValue::Stream(stream) = res.fetch(&src)? {
            stream
        } else {
            return Err(eyre!("Resource value and address types don't match."));
        };

        let frame = Arc::new(Mutex::new(None));
        let volume = self.volume.or(&config.volume()).value();
        let mut stream = stream.cloned(frame.clone(), StreamMode::Normal(self.channel), volume)?;

        if !stream.has_video() && self.width.as_ref().is_some() {
            return Err(eyre!(
                "Video-less stream `?` should not be supplied a width"
            ));
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
        let join_handle = thread::spawn(move || -> Result<()> {
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
            width: self.width.as_f32(),
            looping,
            link_start: tx_start,
            link_stop: Some(rx_stop),
            join_handle: Some(join_handle),
            background: self.background.into(),
            pad: self.pad,
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
        _state: &State,
    ) -> Result<Signal> {
        self.link_start
            .send(())
            .wrap_err("Failed to send start signal to concurrent stream thread.")?;

        let rx_stop = self
            .link_stop
            .take()
            .ok_or_else(|| eyre!("Link to streaming thread could not be acquired for action"))?;

        let done = self.done.clone();
        let join_handle = self
            .join_handle
            .take()
            .ok_or_else(|| eyre!("JoinHandle for action has died prematurely"))?;

        if let Ok(true) = *self.done.lock().unwrap() {
            sync_writer.push(SyncSignal::UpdateGraph);
            return Ok(Signal::none());
        }

        {
            let mut sync_writer = sync_writer.clone();

            thread::spawn(move || {
                let _ = rx_stop.recv();
                *done.lock().unwrap() = match join_handle.join() {
                    Ok(Ok(_)) => Ok(true),
                    Ok(Err(e)) => Err(e).wrap_err("Stream decoder thread failed with error."),
                    Err(_) => Err(eyre!("Failed to graciously close stream decoder thread.")),
                };
                sync_writer.push(SyncSignal::UpdateGraph);
            });
        }

        sync_writer.push(SyncSignal::Repaint);
        Ok(Signal::none())
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Response> {
        let (texture, size) = self
            .frame
            .lock()
            .unwrap()
            .unwrap_or_else(|| (TextureId::default(), Vec2::splat(1.0)));

        let scale = if let Some(width) = self.width {
            width / size.x
        } else {
            1.0
        };

        let response = CentralPanel::default()
            .frame(Frame::default().fill(self.background))
            .show_inside(ui, |ui| {
                if self.pad {
                    ui.centered_and_justified(|ui| ui.image(texture, size * scale))
                        .inner
                } else {
                    ui.image(texture, size * scale)
                }
            })
            .response;

        if self.framerate > 0.0 {
            ui.ctx().request_repaint();
        }

        Ok(response)
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
