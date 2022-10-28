use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{Evaluator, Interpreter, LoggerSignal, ResourceMap, IO};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eyre::{eyre, Context, Error, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Math {
    expr: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    vars: BTreeMap<String, f64>,
    #[serde(default)]
    interpreter: Interpreter,
    #[serde(default)]
    persistent: bool,
    #[serde(default)]
    in_mapping: BTreeMap<SignalId, String>,
    #[serde(default)]
    in_update: SignalId,
    #[serde(default)]
    out_result: SignalId,
}

stateful!(Math {
    _expr: String,
    name: String,
    vars: BTreeMap<String, f64>,
    evaluator: Evaluator,
    persistent: bool,
    in_mapping: BTreeMap<SignalId, String>,
    in_update: SignalId,
    out_result: SignalId,
});

impl Action for Math {
    #[inline]
    fn in_signals(&self) -> BTreeSet<SignalId> {
        let mut signals: BTreeSet<_> = self.in_mapping.keys().cloned().collect();
        signals.insert(self.in_update);
        signals
    }

    #[inline]
    fn out_signals(&self) -> BTreeSet<SignalId> {
        BTreeSet::from([self.out_result])
    }

    #[inline(always)]
    fn init(mut self) -> Result<Box<dyn Action>, Error>
    where
        Self: 'static + Sized,
    {
        self.expr = self.expr.trim().to_owned();
        if self.expr.is_empty() {
            return Err(eyre!("`Math` expression cannot be empty."));
        }

        let re = Regex::new(r"^[[:alpha:]][[:word:]]*$").unwrap();
        for (_, var) in self.in_mapping.iter() {
            if var.as_str() == "self" {
                return Err(eyre!(
                    "Reserved variable (\"self\") of `Math` cannot be included in `in_mapping`."
                ));
            } else if !re.is_match(var) {
                return Err(eyre!("Invalid variable name ({var}) in `in_mapping`."));
            }
        }

        let re = Regex::new(r"(^|[^[:word:]])([[:alpha:]][[:word:]]*)([^[:word:]]|$)").unwrap();
        for caps in re.captures_iter(&self.expr) {
            self.vars.entry(caps[2].to_owned()).or_default();
        }
        self.vars.entry("self".to_owned()).or_default();

        for (_, var) in self.in_mapping.iter() {
            if !self.vars.contains_key(var) {
                return Err(eyre!("Undefined variable ({var}) in `in_mapping`."));
            }
        }

        if self.out_result != 0
            && (self.in_mapping.contains_key(&self.out_result) || self.in_update == self.out_result)
        {
            return Err(eyre!("Recursive expression not allowed."));
        }

        if self.in_update != 0 && self.in_mapping.contains_key(&self.in_update) {
            return Err(eyre!("`in_update` cannot overlap with `in_mapping`."));
        }

        if self.name.is_empty() && self.out_result == 0 {
            return Err(eyre!("`Math` without a `name` or `out_result` is useless."));
        }

        Ok(Box::new(self))
    }

    fn stateful(
        &self,
        _io: &IO,
        _res: &ResourceMap,
        config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        let interpreter = self.interpreter.or(&config.math_interpreter());

        Ok(Box::new(StatefulMath {
            done: false,
            _expr: self.expr.clone(),
            name: self.name.clone(),
            vars: self.vars.clone(),
            evaluator: interpreter.parse(&self.expr)?,
            persistent: self.persistent,
            in_mapping: self.in_mapping.clone(),
            in_update: self.in_update,
            out_result: self.out_result,
        }))
    }
}

impl StatefulAction for StatefulMath {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        if self.persistent { INFINITE } else { DEFAULT }.into()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        for (id, var) in self.in_mapping.iter() {
            if let Some(entry) = self.vars.get_mut(var) {
                if let Some(value) = state.get(id) {
                    *entry = match value {
                        Value::Integer(v) => *v as f64,
                        Value::Float(v) => *v,
                        Value::Bool(v) => {
                            if *v {
                                1.0
                            } else {
                                0.0
                            }
                        }
                        _ => 0.0,
                    };
                }
            }
        }

        if !self.persistent {
            self.done = true;
            sync_writer.push(SyncSignal::UpdateGraph);
        }

        self.eval(async_writer)
            .wrap_err("Failed to initialize mathematical expression.")
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        let mut changed = false;
        if let ActionSignal::StateChanged(_, signal) = signal {
            for id in signal {
                if let Some(var) = self.in_mapping.get(id) {
                    if let Some(entry) = self.vars.get_mut(var) {
                        *entry = match state.get(id).unwrap() {
                            Value::Integer(v) => *v as f64,
                            Value::Float(v) => *v,
                            Value::Bool(v) => {
                                if *v {
                                    1.0
                                } else {
                                    0.0
                                }
                            }
                            _ => 0.0,
                        };
                    }
                    changed = true;
                }
            }

            if signal.contains(&self.in_update) {
                changed = true;
            }
        }

        if !changed {
            return Ok(Signal::none());
        }

        self.eval(async_writer)
            .wrap_err("Failed to update mathematical expression.")
    }

    #[inline]
    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Signal> {
        self.done = true;
        Ok(Signal::none())
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("name", format!("{:?}", self.name))])
            .collect()
    }
}

impl StatefulMath {
    fn eval(&mut self, async_writer: &mut QWriter<AsyncSignal>) -> Result<Signal> {
        let result = self.evaluator.eval(&mut self.vars)?;

        self.vars.insert("self".to_owned(), result);

        if !self.name.is_empty() {
            async_writer.push(LoggerSignal::Append(
                "math".to_owned(),
                (self.name.clone(), Value::Float(result)),
            ));
        }

        if self.out_result > 0 {
            Ok(vec![(self.out_result, Value::Float(result))].into())
        } else {
            Ok(Signal::none())
        }
    }
}
