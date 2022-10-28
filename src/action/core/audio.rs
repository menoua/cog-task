//@ audio

use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{
    drop_channel, interlace_channels, ResourceAddr, ResourceMap, ResourceValue, TimePrecision,
    Trigger, IO,
};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use crate::util::spin_sleeper;
use eyre::{eyre, Context, Result};
use rodio::{Sink, Source};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::BTreeSet;
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
    #[serde(default = "defaults::volume")]
    volume: f32,
    #[serde(default)]
    looping: bool,
    #[serde(default)]
    trigger: Trigger,
    #[serde(default)]
    in_volume: SignalId,
}

stateful_arc!(Audio {
    duration: Duration,
    looping: bool,
    sink: Arc<Mutex<Option<Sink>>>,
    link: Option<(Sender<()>, Receiver<()>)>,
    in_volume: SignalId,
});

mod defaults {
    pub fn volume() -> f32 {
        1.0
    }
}

impl Action for Audio {
    #[inline(always)]
    fn in_signals(&self) -> BTreeSet<SignalId> {
        BTreeSet::from([self.in_volume])
    }

    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<ResourceAddr> {
        if let Trigger::Ext(trig) = &self.trigger {
            vec![
                ResourceAddr::Audio(self.src.to_owned()),
                ResourceAddr::Audio(trig.clone()),
            ]
        } else {
            vec![ResourceAddr::Audio(self.src.to_owned())]
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
        let src = ResourceAddr::Audio(self.src.clone());
        let src = if let ResourceValue::Audio(src) = res.fetch(&src)? {
            src
        } else {
            return Err(eyre!("Resource value and address types don't match."));
        };

        let src = match (&self.trigger, config.use_trigger()) {
            (Trigger::Ext(trig), true) => {
                let trig = ResourceAddr::Audio(trig.clone());
                let trig = if let ResourceValue::Audio(trig) = res.fetch(&trig)? {
                    trig
                } else {
                    return Err(eyre!("Resource value and address types don't match."));
                };

                interlace_channels(src, trig)?
            }
            (Trigger::Int, false) => drop_channel(src)?,
            _ => src,
        };

        let duration = src.total_duration().unwrap();
        let sink = io.audio()?;

        sink.pause();
        sink.set_volume(self.volume * config.base_volume());
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
                        TimePrecision::Inherit => {
                            *done.lock().unwrap() = Err(eyre!(
                                "Invalid state at runtime (time_precision=`Inherit`)."
                            ));
                        }
                        TimePrecision::RespectIntervals => {
                            if let Some(sink) = sink.lock().unwrap().take() {
                                sink.detach();
                            }
                            *done.lock().unwrap() = Ok(true);
                        }
                        TimePrecision::RespectBoundaries => {
                            let mut over = false;
                            let step = Duration::from_micros(50);
                            while !over {
                                if let Some(sink) = sink.lock().unwrap().as_ref() {
                                    if sink.empty() {
                                        over = true;
                                    } else {
                                        sleeper.sleep(step);
                                    }
                                } else {
                                    over = true;
                                }
                            }
                            *done.lock().unwrap() = Ok(true);
                        }
                    }
                }

                let _ = tx_stop.send(());
            });
        }

        Ok(Box::new(StatefulAudio {
            done,
            duration,
            looping: self.looping,
            sink,
            link: Some((tx_start, rx_stop)),
            in_volume: self.in_volume,
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
    ) -> Result<Signal> {
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

        Ok(Signal::none())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        match signal {
            ActionSignal::StateChanged(_, signal) if signal.contains(&self.in_volume) => {}
            _ => return Ok(Signal::none()),
        };

        if let Some(Value::Float(vol)) = state.get(&self.in_volume) {
            let vol = vol.clamp(0.0, 1.0) as f32;
            if let Some(sink) = self.sink.lock().unwrap().as_mut() {
                sink.set_volume(vol);
            }
        }

        Ok(Signal::none())
    }

    #[inline(always)]
    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        if let Some(sink) = self.sink.lock().unwrap().take() {
            sink.stop();
        }
        *self.done.lock().unwrap() = Ok(true);
        Ok(Signal::none())
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
