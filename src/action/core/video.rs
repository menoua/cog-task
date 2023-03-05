//@ stream

use crate::action::{Action, Props, StatefulAction, INFINITE, VISUAL};
use crate::comm::{QWriter, Signal};
use crate::resource::{
    Color, IoManager, OptionalFloat, ResourceAddr, ResourceManager, ResourceValue,
};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use crate::util::spin_sleeper;
use eframe::egui;
use eframe::egui::{CentralPanel, Color32, CursorIcon, Frame, Response, TextureId, Vec2};
use eyre::{eyre, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Video {
    src: PathBuf,
    #[serde(default)]
    width: OptionalFloat,
    #[serde(default)]
    looping: bool,
    #[serde(default)]
    background: Color,
    #[serde(default = "defaults::pad")]
    pad: bool,
}

stateful_arc!(Video {
    frames: Arc<Vec<(TextureId, Vec2)>>,
    framerate: f64,
    duration: Duration,
    position: Arc<Mutex<usize>>,
    width: Option<f32>,
    looping: bool,
    link: Option<(Sender<()>, Receiver<()>)>,
    background: Color32,
    pad: bool,
});

mod defaults {
    pub fn pad() -> bool {
        true
    }
}

impl Action for Video {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<ResourceAddr> {
        vec![ResourceAddr::Video(self.src.clone())]
    }

    fn stateful(
        &self,
        _io: &IoManager,
        res: &ResourceManager,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        let src = ResourceAddr::Video(self.src.clone());
        match res.fetch(&src)? {
            ResourceValue::Video(frames, framerate) => {
                let done = Arc::new(Mutex::new(Ok(frames.is_empty())));
                let position = Arc::new(Mutex::new(0));
                let duration = Duration::from_secs_f64(frames.len() as f64 / framerate);

                let (tx_start, rx_start) = mpsc::channel();
                let (tx_stop, rx_stop) = mpsc::channel();

                {
                    let position = position.clone();
                    let done = done.clone();
                    let sleeper = spin_sleeper();
                    let period = Duration::from_secs_f64(1.0 / framerate);
                    let n_frames = frames.len();
                    let looping = self.looping;

                    thread::spawn(move || {
                        if rx_start.recv().is_err() {
                            return;
                        }

                        loop {
                            sleeper.sleep(period);
                            let mut done = done.lock().unwrap();
                            let mut pos = position.lock().unwrap();
                            if *pos == n_frames - 1 {
                                if looping {
                                    *pos = 0;
                                } else {
                                    *done = Ok(true);
                                }
                            } else {
                                *pos += 1;
                            }
                            if let Ok(true) = *done {
                                break;
                            }
                        }

                        let _ = tx_stop.send(());
                    });
                }

                Ok(Box::new(StatefulVideo {
                    done,
                    frames,
                    framerate,
                    duration,
                    position,
                    width: self.width.as_f32(),
                    looping: self.looping,
                    link: Some((tx_start, rx_stop)),
                    background: self.background.into(),
                    pad: self.pad,
                }))
            }
            _ => Err(eyre!(
                "Video action supplied non-video resource: `{:?}`",
                self.src
            )),
        }
    }
}

impl StatefulAction for StatefulVideo {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        if self.looping {
            INFINITE | VISUAL
        } else {
            VISUAL
        }
        .into()
    }

    #[inline]
    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        let link = self
            .link
            .take()
            .ok_or_else(|| eyre!("Link to video thread could not be acquired for action",))?;

        link.0
            .send(())
            .wrap_err("Failed to send start signal to concurrent video thread.")?;

        if let Ok(true) = *self.done.lock().unwrap() {
            sync_writer.push(SyncSignal::UpdateGraph);
            return Ok(Signal::none());
        }

        {
            let done = self.done.clone();
            let mut sync_writer = sync_writer.clone();

            thread::spawn(move || {
                let link = link;
                let _ = link.1.recv();
                *done.lock().unwrap() = Ok(true);
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
        let (texture, size) = self.frames[*self.position.lock().unwrap()];

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

        ui.ctx().request_repaint();
        Ok(response)
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([
                ("duration", format!("{:?}", self.duration)),
                ("framerate", format!("{:?}", self.framerate)),
                ("looping", format!("{:?}", self.looping)),
            ])
            .collect()
    }
}
