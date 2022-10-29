use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{IoManager, Key, LoggerSignal, ResourceManager};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eyre::{eyre, Error, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reaction {
    times: Vec<f32>,
    #[serde(default = "defaults::group")]
    group: String,
    #[serde(default = "defaults::keys")]
    keys: BTreeSet<Key>,
    #[serde(default = "defaults::tol")]
    tol: f32,
    #[serde(default)]
    out_rt: SignalId,
    #[serde(default)]
    out_accuracy: SignalId,
    #[serde(default)]
    out_mean_rt: SignalId,
    #[serde(default)]
    out_recall: SignalId,
}

stateful!(Reaction {
    group: String,
    keys: BTreeSet<Key>,
    times: Vec<Duration>,
    tol: Duration,
    since: Instant,
    next: Option<usize>,
    reaction_correct: Vec<bool>,
    reaction_times: Vec<f32>,
    reaction_rts: Vec<f32>,
    out_rt: SignalId,
    out_accuracy: SignalId,
    out_mean_rt: SignalId,
    out_recall: SignalId,
});

mod defaults {
    use crate::resource::Key;
    use std::collections::BTreeSet;

    #[inline(always)]
    pub fn group() -> String {
        "reaction".to_owned()
    }

    #[inline(always)]
    pub fn keys() -> BTreeSet<Key> {
        BTreeSet::new()
    }

    #[inline(always)]
    pub fn tol() -> f32 {
        2.0
    }
}

impl Action for Reaction {
    #[inline]
    fn out_signals(&self) -> BTreeSet<SignalId> {
        BTreeSet::from([
            self.out_rt,
            self.out_accuracy,
            self.out_mean_rt,
            self.out_recall,
        ])
    }

    #[inline(always)]
    fn init(mut self) -> Result<Box<dyn Action>, Error>
    where
        Self: 'static + Sized,
    {
        if self.group.is_empty() {
            return Err(eyre!("Reaction `group` cannot be an empty string"));
        }

        self.times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        Ok(Box::new(self))
    }

    fn stateful(
        &self,
        _io: &IoManager,
        _res: &ResourceManager,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Ok(Box::new(StatefulReaction {
            done: false,
            group: self.group.clone(),
            keys: self.keys.clone(),
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
            out_rt: self.out_rt,
            out_accuracy: self.out_accuracy,
            out_recall: self.out_recall,
            out_mean_rt: self.out_mean_rt,
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
        _state: &State,
    ) -> Result<Signal> {
        self.since = Instant::now();
        async_writer.push(LoggerSignal::Append(
            self.group.clone(),
            ("event".to_owned(), Value::Text("start".to_owned())),
        ));
        Ok(Signal::none())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        let (time, keys) = match signal {
            ActionSignal::KeyPress(t, k) => (t.duration_since(self.since), k),
            _ => return Ok(Signal::none()),
        };

        if !self.keys.is_empty() && keys.is_disjoint(&self.keys) {
            return Ok(Signal::none());
        }

        self.reaction_times.push(time.as_secs_f32());

        let mut correct = false;
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
                    let rt = (time - target).as_secs_f32();
                    self.reaction_correct.push(true);
                    self.reaction_rts.push(rt);
                    if i < self.times.len() - 1 {
                        self.next = Some(i + 1);
                    } else {
                        self.next = None;
                    }
                    if self.out_rt > 0 {
                        sync_writer.push(SyncSignal::Emit(
                            Instant::now(),
                            vec![(self.out_rt, Value::Float(rt as f64))].into(),
                        ))
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
                Value::Array(vec![
                    Value::Float(self.reaction_times[self.reaction_times.len() - 1] as f64),
                    Value::Float(self.reaction_rts[self.reaction_rts.len() - 1] as f64),
                ]),
            )
        } else {
            (
                "incorrect".to_string(),
                Value::Array(vec![Value::Float(
                    self.reaction_times[self.reaction_times.len() - 1] as f64,
                )]),
            )
        };
        async_writer.push(LoggerSignal::Append(self.group.clone(), entry));

        Ok(Signal::none())
    }

    #[inline]
    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        let accuracy = self.accuracy();
        let mean_rt = self.mean_rt();
        let recall = self.recall();

        async_writer.push(LoggerSignal::Extend(
            self.group.clone(),
            vec![
                ("event".to_owned(), Value::Text("stop".to_owned())),
                ("accuracy".to_owned(), Value::Float(accuracy)),
                ("mean_rt".to_owned(), Value::Float(mean_rt)),
                ("recall".to_owned(), Value::Float(recall)),
            ],
        ));

        let mut news = vec![];
        if self.out_accuracy > 0 {
            news.push((self.out_accuracy, Value::Float(accuracy)))
        }
        if self.out_mean_rt > 0 {
            news.push((self.out_mean_rt, Value::Float(mean_rt)))
        }
        if self.out_recall > 0 {
            news.push((self.out_recall, Value::Float(recall)))
        }
        Ok(news.into())
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("group", format!("{:?}", self.group))])
            .collect()
    }
}

impl StatefulReaction {
    #[inline(always)]
    fn accuracy(&self) -> f64 {
        self.reaction_rts.len() as f64 / self.reaction_correct.len() as f64
    }

    #[inline(always)]
    fn recall(&self) -> f64 {
        self.reaction_rts.len() as f64 / self.times.len() as f64
    }

    #[inline(always)]
    fn mean_rt(&self) -> f64 {
        self.reaction_rts.iter().sum::<f32>() as f64 / self.reaction_rts.len() as f64
    }
}
