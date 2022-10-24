use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE};
use crate::config::{Config, TimePrecision};
use crate::io::IO;
use crate::queue::QWriter;
use crate::resource::audio::{drop_channel, interlace_channels, Trigger};
use crate::resource::{ResourceMap, ResourceValue};
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::scheduler::State;
use crate::signal::SignalId;
use crate::util::spin_sleeper;
use eframe::egui::Ui;
use eyre::{eyre, Context, Result};
use rodio::{Sink, Source};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Audio {
    src: PathBuf,
    #[serde(default)]
    volume: Option<f32>,
    #[serde(default)]
    looping: bool,
    #[serde(default)]
    trigger: Trigger,
    #[serde(default)]
    volume_control: SignalId,
}

stateful_arc!(Audio {
    duration: Duration,
    looping: bool,
    sink: Arc<Mutex<Option<Sink>>>,
    link: Option<(Sender<()>, Receiver<()>)>,
    volume_control: SignalId,
});

impl Action for Audio {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        if let Trigger::Ext(trig) = &self.trigger {
            vec![self.src.to_owned(), trig.clone()]
        } else {
            vec![self.src.to_owned()]
        }
    }

    fn stateful(
        &self,
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        let src = if let ResourceValue::Audio(src) = res.fetch(&self.src)? {
            src
        } else {
            return Err(eyre!(
                "Audio action supplied non-audio resource: `{:?}`",
                self.src
            ));
        };

        let src = match (&self.trigger, config.use_trigger()) {
            (Trigger::Ext(trig), true) => {
                let trig = if let ResourceValue::Audio(trig) = res.fetch(trig)? {
                    trig
                } else {
                    return Err(eyre!(
                        "Audio action supplied non-audio trigger resource: `{trig:?}`"
                    ));
                };

                interlace_channels(src, trig)?
            }
            (Trigger::Int, false) => drop_channel(src)?,
            _ => src,
        };

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

        Ok(Box::new(StatefulAudio {
            done,
            duration,
            looping: self.looping,
            sink,
            link: Some((tx_start, rx_stop)),
            volume_control: self.volume_control,
        }))
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
        _state: &State,
    ) -> Result<()> {
        let link = self
            .link
            .take()
            .ok_or_else(|| eyre!("Link to audio thread could not be acquired for action."))?;

        link.0
            .send(())
            .wrap_err("Failed to send start signal to concurrent audio thread.")?;

        if let Ok(true) = *self.done.lock().unwrap() {
            sync_writer.push(SyncSignal::UpdateGraph);
        } else {
            let done = self.done.clone();
            let mut sync_writer = sync_writer.clone();
            thread::spawn(move || {
                let link = link;
                let _ = link.1.recv();
                *done.lock().unwrap() = Ok(true);
                sync_writer.push(SyncSignal::UpdateGraph);
            });
        }

        Ok(())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<()> {
        let volume_control = match self.volume_control {
            SignalId::Internal(i) => i,
            _ => return Ok(()),
        };

        let signal = match signal {
            ActionSignal::Internal(_, signal) => signal,
            _ => return Ok(()),
        };

        if let Some(Value::Float(vol)) = signal.get(&volume_control) {
            let vol = vol.clamp(0.0, 1.0) as f32;
            if let Some(sink) = self.sink.lock().unwrap().as_mut() {
                sink.set_volume(vol);
            }
        }

        Ok(())
    }

    fn show(
        &mut self,
        _ui: &mut Ui,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<()> {
        Ok(())
    }

    #[inline(always)]
    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<()> {
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
