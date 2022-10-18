use crate::action::{Action, ActionEnum, Props, StatefulAction, StatefulActionEnum, DEFAULT, INFINITE, ActionSignal};
use crate::config::{Config, TimePrecision};
use crate::error;
use crate::error::Error::{InternalError, InvalidResourceError};
use crate::io::IO;
use crate::resource::{ResourceMap, ResourceValue};
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use crate::util::spin_sleeper;
use rodio::{Sink, Source};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use eframe::egui::Ui;
use crate::error::Error;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Audio {
    src: PathBuf,
    #[serde(default)]
    volume: Option<f32>,
    #[serde(default)]
    looping: bool,
}

stateful_arc!(Audio {
    duration: Duration,
    looping: bool,
    sink: Arc<Mutex<Option<Sink>>>,
    link: Option<(Sender<()>, Receiver<()>)>,
});

impl Action for Audio {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![self.src.to_owned()]
    }

    fn stateful(
        &self,
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<StatefulActionEnum, error::Error> {
        if let ResourceValue::Audio(src) = res.fetch(&self.src)? {
            let duration = src.total_duration().unwrap();
            let sink = io.audio()?;

            sink.pause();
            if let Some(volume) = config.volume(self.volume) {
                sink.set_volume(volume);
            }
            if self.looping {
                sink.append(src.repeat_infinite())
            } else {
                sink.append(src)
            }

            let done = Arc::new(Mutex::new(Ok(sink.empty())));
            let sink = Arc::new(Mutex::new(Some(sink)));
            let (tx_start, rx_start) = mpsc::channel();
            let (tx_stop, rx_stop) = mpsc::channel();

            {
                let done = done.clone();
                let sink = sink.clone();
                let time_precision = config.time_precision();
                let looping = self.looping;
                let sleeper = spin_sleeper();

                thread::spawn(move || {
                    if rx_start.recv().is_err() {
                        return;
                    }

                    if let Some(sink) = sink.lock().unwrap().as_ref() {
                        sink.play();
                    } else {
                        let _ = tx_stop.send(());
                        return;
                    }

                    if looping {
                        loop {
                            thread::sleep(Duration::from_secs(5));
                            if let Err(TryRecvError::Disconnected) = rx_start.try_recv() {
                                break;
                            }
                        }
                    } else {
                        // wait for the exact duration of the audio (note that the actual audio might
                        // take longer to finish playing due to IO delay, etc.), leaving what remains
                        // to be played in a serial or parallel mode depending on time_precision conf
                        let target_time = Instant::now() + duration;
                        sleeper.sleep(target_time - Instant::now());

                        match time_precision {
                            TimePrecision::RespectIntervals => {
                                if let Some(sink) = sink.lock().unwrap().take() {
                                    sink.detach();
                                }
                            }
                            TimePrecision::RespectBoundaries => {
                                let mut done = false;
                                let step = Duration::from_micros(50);
                                while !done {
                                    if let Some(sink) = sink.lock().unwrap().as_ref() {
                                        if sink.empty() {
                                            done = true;
                                        } else {
                                            sleeper.sleep(step);
                                        }
                                    } else {
                                        done = true;
                                    }
                                }
                            }
                        }
                    }

                    *done.lock().unwrap() = Ok(true);
                    let _ = tx_stop.send(());
                });
            }

            Ok(StatefulAudio {
                id: 0,
                done,
                duration,
                looping: self.looping,
                sink,
                link: Some((tx_start, rx_stop)),
            }
            .into())
        } else {
            Err(InvalidResourceError(format!(
                "Audio action supplied non-audio resource: `{:?}`",
                self.src
            )))
        }
    }
}

impl StatefulAction for StatefulAudio {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        if self.looping { INFINITE } else { DEFAULT }.into()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        let link = self.link.take().ok_or_else(|| {
            InternalError(format!(
                "Link to audio thread could not be acquired for action `{}`",
                self.id
            ))
        })?;

        link.0.send(()).map_err(|e| {
            InternalError(format!(
                "Failed to send start signal to concurrent audio thread:\n{e:#?}"
            ))
        })?;

        let done = self.done.clone();
        let mut sync_writer = sync_writer.clone();
        thread::spawn(move || {
            let link = link;
            let _ = link.1.recv();
            *done.lock().unwrap() = Ok(true);
            sync_writer.push(SyncSignal::UpdateGraph);
        });

        Ok(())
    }

    fn update(&mut self, signal: &ActionSignal, sync_writer: &mut QWriter<SyncSignal>, async_writer: &mut QWriter<AsyncSignal>) -> Result<(), Error> {
        Ok(())
    }

    fn show(&mut self, ui: &mut Ui, sync_writer: &mut QWriter<SyncSignal>, async_writer: &mut QWriter<AsyncSignal>) -> Result<(), Error> {
        Ok(())
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        if let Some(sink) = self.sink.lock().unwrap().take() {
            sink.stop();
        }
        *self.done.lock().unwrap() = Ok(true);
        Ok(())
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("duration", format!("{:?}", self.duration))])
            .collect()
    }
}

impl Drop for StatefulAudio {
    fn drop(&mut self) {
        if let Some(sink) = self.sink.lock().unwrap().take() {
            sink.stop();
        }
    }
}
