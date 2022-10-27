//@ stream

use crate::action::{Action, Props, StatefulAction, DEFAULT, INFINITE, VISUAL};
use crate::comm::{QWriter, Signal};
use crate::resource::{Color, MediaMode, ResourceAddr, ResourceMap, ResourceValue, Trigger};
use crate::server::{AsyncSignal, Config, State, SyncSignal, IO};
use crate::util::spin_sleeper;
use eframe::egui;
use eframe::egui::{CentralPanel, Color32, CursorIcon, Frame, TextureId, Vec2};
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
    width: Option<u16>,
    #[serde(default = "defaults::volume")]
    volume: f32,
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
    join_handle: Option<JoinHandle<Result<()>>>,
    background: Color32,
});

mod defaults {
    pub fn volume() -> f32 {
        1.0
    }
}

impl Action for Stream {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<ResourceAddr> {
        if let Trigger::Ext(trig) = &self.trigger {
            vec![
                ResourceAddr::Stream(self.src.clone()),
                ResourceAddr::Ref(trig.clone()),
            ]
        } else {
            vec![ResourceAddr::Stream(self.src.clone())]
        }
    }

    #[inline]
    fn init(self) -> Result<Box<dyn Action>> {
        if !(0.0..=1.0).contains(&self.volume) {
            return Err(eyre!(
                "Stream volume should be a float number between 0.0 and 1.0"
            ));
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
    ) -> Result<Box<dyn StatefulAction>> {
        let src = ResourceAddr::Stream(self.src.clone());
        let stream = if let ResourceValue::Stream(stream) = res.fetch(&src)? {
            stream
        } else {
            return Err(eyre!("Resource value and address types don't match."));
        };

        let use_trigger = config.use_trigger();
        let media_mode = match (&self.trigger, use_trigger) {
            (Trigger::Ext(trig), true) => {
                let trig = ResourceAddr::Ref(trig.clone());
                let trig = if let ResourceValue::Ref(trig) = res.fetch(&trig)? {
                    trig
                } else {
                    return Err(eyre!("Resource value and address types don't match."));
                };
                MediaMode::WithExtTrigger(trig)
            }
            (Trigger::Int, false) => MediaMode::SansIntTrigger,
            _ => MediaMode::Normal,
        };

        let frame = Arc::new(Mutex::new(None));
        let volume = self.volume * config.base_volume();
        let mut stream = stream.cloned(frame.clone(), media_mode, volume)?;

        if !stream.has_video() && self.width.is_some() {
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
            width: self.width,
            looping,
            link_start: tx_start,
            link_stop: Some(rx_stop),
            join_handle: Some(join_handle),
            background: self.background.into(),
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
    ) -> Result<()> {
        let (texture, size) = self
            .frame
            .lock()
            .unwrap()
            .unwrap_or_else(|| (TextureId::default(), Vec2::splat(1.0)));

        ui.output().cursor_icon = CursorIcon::None;

        CentralPanel::default()
            .frame(Frame::default().fill(self.background))
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
        _state: &State,
    ) -> Result<Signal> {
        *self.done.lock().unwrap() = Ok(true);
        if self.framerate > 0.0 {
            sync_writer.push(SyncSignal::Repaint);
        }
        Ok(Signal::none())
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
