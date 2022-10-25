use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, SignalId};
use crate::resource::{Key, ResourceMap};
use crate::server::{AsyncSignal, Config, LoggerSignal, State, SyncSignal, IO};
use eframe::egui;
use eframe::egui::Ui;
use eyre::{eyre, Error, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::HashSet;
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
    sig_accuracy: SignalId,
    #[serde(default)]
    sig_recall: SignalId,
    #[serde(default)]
    sig_mean_rt: SignalId,
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
    sig_accuracy: SignalId,
    sig_recall: SignalId,
    sig_mean_rt: SignalId,
});

mod defaults {
    use crate::resource::Key;
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
    ) -> Result<Box<dyn StatefulAction>> {
        if self.group.is_empty() {
            return Err(eyre!("Reaction `group` cannot be an empty string"));
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
            sig_accuracy: self.sig_accuracy,
            sig_recall: self.sig_recall,
            sig_mean_rt: self.sig_mean_rt,
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
    ) -> Result<(), Error> {
        self.since = Instant::now();
        async_writer.push(LoggerSignal::Append(
            self.group.clone(),
            ("event".to_owned(), Value::Text("start".to_owned())),
        ));
        Ok(())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<()> {
        let (time, keys) = match signal {
            ActionSignal::KeyPress(t, k) => (t.duration_since(self.since), k),
            _ => return Ok(()),
        };

        if !self.keys.is_empty() && keys.is_disjoint(&self.keys) {
            return Ok(());
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

        Ok(())
    }

    fn show(
        &mut self,
        _ui: &mut Ui,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        Ok(())
    }

    #[inline]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<()> {
        let accuracy = self.accuracy();
        let recall = self.recall();
        let mean_rt = self.mean_rt();

        async_writer.push(LoggerSignal::Extend(
            self.group.clone(),
            vec![
                ("event".to_owned(), Value::Text("stop".to_owned())),
                ("accuracy".to_owned(), Value::Float(accuracy)),
                ("recall".to_owned(), Value::Float(recall)),
                ("mean_rt".to_owned(), Value::Float(mean_rt)),
            ],
        ));

        let mut signals = vec![];
        if !self.sig_accuracy.is_none() {
            signals.push((self.sig_accuracy, Value::Float(accuracy)))
        }
        if !self.sig_recall.is_none() {
            signals.push((self.sig_recall, Value::Float(recall)))
        }
        if !self.sig_mean_rt.is_none() {
            signals.push((self.sig_mean_rt, Value::Float(mean_rt)))
        }
        if !signals.is_empty() {
            sync_writer.push(SyncSignal::Emit(Instant::now(), signals.into()));
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
