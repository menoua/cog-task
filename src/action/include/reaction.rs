use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::error::Error::InvalidNameError;
use crate::io::IO;
use crate::logger::LoggerSignal;
use crate::message::InternalSignal;
use crate::resource::key::Key;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use eframe::egui;
use eframe::egui::Ui;
use ron::{Number, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reaction {
    times: Vec<f32>,
    #[serde(default = "defaults::group")]
    group: String,
    #[serde(default = "defaults::keys")]
    keys: HashSet<Key>,
    #[serde(default = "defaults::tol")]
    tol: f32,
    #[serde(default)]
    outgoing: Option<String>,
}

stateful!(Reaction {
    group: String,
    keys: HashSet<egui::Key>,
    times: Vec<Duration>,
    tol: Duration,
    since: Instant,
    next: Option<usize>,
    reaction_correct: Vec<bool>,
    reaction_times: Vec<f32>,
    reaction_rts: Vec<f32>,
    outgoing: Option<String>,
});

mod defaults {
    use crate::resource::key::Key;
    use std::collections::HashSet;

    #[inline(always)]
    pub fn group() -> String {
        "reaction".to_owned()
    }

    #[inline(always)]
    pub fn keys() -> HashSet<Key> {
        HashSet::new()
    }

    #[inline(always)]
    pub fn tol() -> f32 {
        2.0
    }
}

impl Action for Reaction {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![]
    }

    #[inline(always)]
    fn init(mut self) -> Result<Box<dyn Action>, Error>
    where
        Self: 'static + Sized,
    {
        self.times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        Ok(Box::new(self))
    }

    fn stateful(
        &self,
        _io: &IO,
        _res: &ResourceMap,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        if self.group.is_empty() {
            return Err(InvalidNameError(
                "Reaction `group` cannot be an empty string".to_owned(),
            ));
        }

        Ok(Box::new(StatefulReaction {
            done: false,
            group: self.group.clone(),
            keys: self.keys.iter().map(|k| k.into()).collect(),
            times: self
                .times
                .iter()
                .map(|t| Duration::from_secs_f32(*t))
                .collect(),
            tol: Duration::from_secs_f32(self.tol),
            since: Instant::now(),
            next: Some(0),
            reaction_correct: vec![],
            reaction_times: vec![],
            reaction_rts: vec![],
            outgoing: self.outgoing.clone(),
        }))
    }
}

impl StatefulAction for StatefulReaction {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        INFINITE.into()
    }

    fn start(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        self.since = Instant::now();
        async_writer.push(LoggerSignal::Append(
            self.group.clone(),
            ("event".to_owned(), Value::String("start".to_owned())),
        ));
        Ok(())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        if let ActionSignal::KeyPress(time, keys) = signal {
            if self.keys.is_empty() || !keys.is_disjoint(&self.keys) {
                let group = self.group.clone();
                let time = time.duration_since(self.since);

                let mut correct = false;
                self.reaction_times.push(time.as_secs_f32());
                if self.next.is_none() {
                    self.reaction_correct.push(false);
                } else {
                    while let Some(i) = self.next {
                        let target = self.times[i];
                        if time < target {
                            self.reaction_correct.push(false);
                            break;
                        } else if time <= target + self.tol {
                            correct = true;
                            self.reaction_correct.push(true);
                            self.reaction_rts.push((time - target).as_secs_f32());
                            if i < self.times.len() - 1 {
                                self.next = Some(i + 1);
                            } else {
                                self.next = None;
                            }
                            break;
                        } else if i < self.times.len() - 1 {
                            self.next = Some(i + 1);
                        } else {
                            self.next = None;
                            self.reaction_correct.push(false);
                            break;
                        }
                    }
                }

                let entry = if correct {
                    (
                        "correct".to_string(),
                        Value::Seq(vec![
                            Value::Number(Number::from(
                                self.reaction_times[self.reaction_times.len() - 1] as f64,
                            )),
                            Value::Number(Number::from(
                                self.reaction_rts[self.reaction_rts.len() - 1] as f64,
                            )),
                        ]),
                    )
                } else {
                    (
                        "incorrect".to_string(),
                        Value::Seq(vec![Value::Number(Number::from(
                            self.reaction_times[self.reaction_times.len() - 1] as f64,
                        ))]),
                    )
                };
                async_writer.push(LoggerSignal::Append(group, entry));
            }
        }
        Ok(())
    }

    fn show(
        &mut self,
        _ui: &mut Ui,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        Ok(())
    }

    #[inline]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        async_writer.push(LoggerSignal::Append(
            self.group.clone(),
            ("event".to_owned(), Value::String("stop".to_owned())),
        ));

        let time = Instant::now();
        if let Some(group) = &self.outgoing {
            let accuracy = self.reaction_rts.len() as f32 / self.reaction_correct.len() as f32;
            let recall = self.reaction_rts.len() as f32 / self.times.len() as f32;
            let mean_rt = self.reaction_rts.iter().sum::<f32>() / self.reaction_rts.len() as f32;

            sync_writer.push(SyncSignal::Internal(
                time,
                InternalSignal::new(vec![
                    (
                        format!("{group}:accuracy"),
                        Value::Number(Number::from(accuracy as f64)),
                    ),
                    (
                        format!("{group}:recall"),
                        Value::Number(Number::from(recall as f64)),
                    ),
                    (
                        format!("{group}:mean_rt"),
                        Value::Number(Number::from(mean_rt as f64)),
                    ),
                ]),
            ));
        }

        self.done = true;
        Ok(())
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("group", format!("{:?}", self.group))])
            .collect()
    }
}
