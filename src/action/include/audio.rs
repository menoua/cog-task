use crate::action::{Action, StatefulAction};
use crate::assets::{SPIN_DURATION, SPIN_STRATEGY};
use crate::signal::QWriter;
use crate::config::{Config, TimePrecision};
use crate::error;
use crate::error::Error::{InternalError, InvalidResourceError};
use crate::io::IO;
use crate::resource::{ResourceMap, ResourceValue};
use crate::scheduler::{AsyncCallback, SyncCallback};
use rodio::{Sink, Source};
use serde::{Deserialize, Serialize};
use spin_sleep::SpinSleeper;
use std::fmt::{Debug, Formatter};
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
        id: usize,
        res: &ResourceMap,
        config: &Config,
        io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
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
                let sleeper = SpinSleeper::new(SPIN_DURATION).with_spin_strategy(SPIN_STRATEGY);

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
                id,
                done,
                duration,
                looping: self.looping,
                sink,
                link: Some((tx_start, rx_stop)),
            }))
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
    fn is_visual(&self) -> bool {
        false
    }

    #[inline(always)]
    fn is_static(&self) -> bool {
        self.looping
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        if let Some(sink) = self.sink.lock().unwrap().take() {
            sink.stop();
        }
        *self.done.lock().unwrap() = Ok(true);
        Ok(())
    }

    fn start(
        &mut self,
        sync_qw: &mut QWriter<SyncCallback>,
        _async_qw: &mut QWriter<AsyncCallback>,
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
        let mut sync_qw = sync_qw.clone();
        thread::spawn(move || {
            let link = link;
            let _ = link.1.recv();
            *done.lock().unwrap() = Ok(true);
            sync_qw.push(SyncCallback::UpdateGraph);
        });

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
