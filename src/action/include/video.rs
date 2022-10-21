use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE, VISUAL};
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::error::Error::{InternalError, InvalidResourceError};
use crate::io::IO;
use crate::resource::color::Color;
use crate::resource::{ResourceMap, ResourceValue};
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use crate::util::spin_sleeper;
use eframe::egui;
use eframe::egui::{CentralPanel, CursorIcon, Frame, TextureId, Vec2};
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
    width: Option<u16>,
    #[serde(default)]
    looping: bool,
    #[serde(default)]
    background: Color,
}

stateful_arc!(Video {
    frames: Arc<Vec<(TextureId, Vec2)>>,
    framerate: f64,
    duration: Duration,
    position: Arc<Mutex<usize>>,
    width: Option<u16>,
    looping: bool,
    link: Option<(Sender<()>, Receiver<()>)>,
    background: Color,
});

impl Video {
    #[inline(always)]
    fn src(&self) -> PathBuf {
        PathBuf::from(self.src.to_str().unwrap())
    }
}

impl Action for Video {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![self.src()]
    }

    fn stateful(
        &self,
        _io: &IO,
        res: &ResourceMap,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        match res.fetch(&self.src())? {
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
                    width: self.width,
                    looping: self.looping,
                    link: Some((tx_start, rx_stop)),
                    background: self.background,
                }))
            }
            _ => Err(InvalidResourceError(format!(
                "Video action supplied non-video resource: `{:?}`",
                self.src
            ))),
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
    ) -> Result<(), error::Error> {
        let link = self.link.take().ok_or_else(|| {
            InternalError(format!(
                "Link to video thread could not be acquired for action",
            ))
        })?;

        link.0.send(()).map_err(|e| {
            InternalError(format!(
                "Failed to send start signal to concurrent video thread:\n{e:#?}"
            ))
        })?;

        if let Ok(true) = *self.done.lock().unwrap() {
            sync_writer.push(SyncSignal::UpdateGraph);
            return Ok(());
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
        let (texture, size) = self.frames[*self.position.lock().unwrap()];

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

        ui.ctx().request_repaint();
        Ok(())
    }

    #[inline(always)]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        *self.done.lock().unwrap() = Ok(true);
        sync_writer.push(SyncSignal::Repaint);
        Ok(())
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
