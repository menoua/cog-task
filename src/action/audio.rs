use crate::action::{Action, StatefulAction};
use crate::config::{Config, TimePrecision};
use crate::error;
use crate::error::Error::InvalidResourceError;
use crate::io::IO;
use crate::resource::{ResourceMap, ResourceValue};
use crate::scheduler::{SchedulerMsg, SPIN_DURATION, SPIN_STRATEGY};
use crate::server::ServerMsg;
use iced::Command;
use rodio::{Sink, Source};
use serde::{Deserialize, Serialize};
use spin_sleep::SpinSleeper;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Audio {
    src: PathBuf,
    #[serde(default)]
    gain: Option<f32>,
    #[serde(default)]
    looping: bool,
}

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
            let src = src.clone();
            let duration = src.total_duration().unwrap();
            let sink = io.audio()?;

            sink.pause();
            match (self.gain, self.looping) {
                (Some(gain), true) => sink.append(src.amplify(gain).repeat_infinite()),
                (Some(gain), false) => sink.append(src.amplify(gain)),
                (None, true) => sink.append(src.repeat_infinite()),
                (None, false) => sink.append(src),
            }

            Ok(Box::new(StatefulAudio {
                id,
                done: false,
                duration,
                looping: self.looping,
                time_precision: config.time_precision(),
                sink: Arc::new(Mutex::new(Some(sink))),
            }))
        } else {
            Err(InvalidResourceError(format!(
                "Audio action supplied non-audio resource: `{:?}`",
                self.src
            )))
        }
    }
}

pub struct StatefulAudio {
    id: usize,
    done: bool,
    duration: Duration,
    looping: bool,
    time_precision: TimePrecision,
    sink: Arc<Mutex<Option<Sink>>>,
}

impl Debug for StatefulAudio {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stateful audio of length {:?}", self.duration)
    }
}

impl StatefulAction for StatefulAudio {
    #[inline(always)]
    fn id(&self) -> usize {
        self.id
    }

    fn is_over(&self) -> bool {
        if self.done {
            true
        } else if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.empty()
        } else {
            true
        }
    }

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
        self.done = true;
        Ok(())
    }

    fn start(&mut self) -> Result<Command<ServerMsg>, error::Error> {
        let sink = self.sink.clone();
        let duration = self.duration;
        let time_precision = self.time_precision;

        if self.looping {
            if let Some(sink) = sink.lock().unwrap().as_ref() {
                sink.play();
                Ok(Command::none())
            } else {
                Ok(Command::perform(async {}, |()| {
                    SchedulerMsg::Advance.wrap()
                }))
            }
        } else {
            Ok(Command::perform(
                async move {
                    let playing = if let Some(sink) = sink.lock().unwrap().as_ref() {
                        sink.play();
                        true
                    } else {
                        false
                    };

                    // wait for the exact duration of the audio (note that the actual audio might
                    // take longer to finish playing due to IO delay, etc.), leaving what remains
                    // to be played in a serial or parallel mode depending on time_precision conf
                    let sleeper = SpinSleeper::new(SPIN_DURATION).with_spin_strategy(SPIN_STRATEGY);
                    if playing {
                        let target_time = Instant::now() + duration;
                        sleeper.sleep(target_time - Instant::now());
                    }

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
                    SchedulerMsg::Advance
                },
                SchedulerMsg::wrap,
            ))
        }
    }
}

impl Drop for StatefulAudio {
    fn drop(&mut self) {
        if let Some(sink) = self.sink.lock().unwrap().take() {
            sink.stop();
        }
    }
}
