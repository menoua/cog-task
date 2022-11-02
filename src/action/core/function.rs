use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{
    Evaluator, Interpreter, IoManager, LoggerSignal, OptionalPath, OptionalString, ResourceAddr,
    ResourceManager, ResourceValue,
};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eyre::{eyre, Context, Error, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Function {
    #[serde(default)]
    name: String,
    #[serde(default)]
    expr: OptionalString,
    #[serde(default)]
    src: OptionalPath,
    #[serde(default)]
    init_expr: OptionalString,
    #[serde(default)]
    init_src: OptionalPath,
    #[serde(default)]
    vars: BTreeMap<String, Value>,
    #[serde(default)]
    interpreter: Interpreter,
    #[serde(default = "defaults::on_start")]
    on_start: bool,
    #[serde(default = "defaults::on_change")]
    on_change: bool,
    #[serde(default)]
    once: bool,
    #[serde(default)]
    in_mapping: BTreeMap<SignalId, String>,
    #[serde(default)]
    in_update: SignalId,
    #[serde(default)]
    lo_response: SignalId,
    #[serde(default)]
    out_result: SignalId,
}

stateful!(Function {
    name: String,
    vars: BTreeMap<String, Value>,
    evaluator: Evaluator,
    on_start: bool,
    on_change: bool,
    once: bool,
    in_mapping: BTreeMap<SignalId, String>,
    in_update: SignalId,
    lo_response: SignalId,
    out_result: SignalId,
});

mod defaults {
    pub fn on_start() -> bool {
        true
    }

    pub fn on_change() -> bool {
        true
    }
}

impl Action for Function {
    #[inline(always)]
    fn init(self) -> Result<Box<dyn Action>, Error>
    where
        Self: 'static + Sized,
    {
        match (self.expr.is_some(), self.src.is_some()) {
            (false, false) => Err(eyre!("`expr` and `src` cannot both be empty."))?,
            (true, true) => Err(eyre!("Only one of `expr` and `src` should be set."))?,
            _ => {}
        };

        if self.init_expr.is_some() && self.init_src.is_some() {
            return Err(eyre!(
                "Only one of `init_expr` and `init_src` should be set."
            ));
        }

        let re = Regex::new(r"^[[:alpha:]][[:word:]]*$").unwrap();
        for (_, var) in self.in_mapping.iter() {
            if var.as_str() == "self" {
                return Err(eyre!(
                    "Reserved variable (\"self\") of Fn cannot be included in `in_mapping`."
                ));
            } else if !re.is_match(var) {
                return Err(eyre!("Invalid variable name ({var}) in `in_mapping`."));
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

        Ok(Box::new(self))
    }

    #[inline]
    fn in_signals(&self) -> BTreeSet<SignalId> {
        let mut signals: BTreeSet<_> = self.in_mapping.keys().cloned().collect();
        signals.extend([self.in_update, self.lo_response]);
        signals
    }

    #[inline]
    fn out_signals(&self) -> BTreeSet<SignalId> {
        BTreeSet::from([self.lo_response, self.out_result])
    }

    fn resources(&self, _config: &Config) -> Vec<ResourceAddr> {
        let mut resources = vec![];
        if let OptionalPath::Some(src) = &self.src {
            resources.push(ResourceAddr::Text(src.clone()));
        }
        if let OptionalPath::Some(src) = &self.init_src {
            resources.push(ResourceAddr::Text(src.clone()));
        }
        resources
    }

    fn stateful(
        &self,
        _io: &IoManager,
        res: &ResourceManager,
        config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        let interpreter = self.interpreter.or(&config.interpreter());

        let init = if let OptionalPath::Some(src) = &self.init_src {
            match res.fetch(&ResourceAddr::Text(src.clone()))? {
                ResourceValue::Text(expr) => (*expr).clone(),
                _ => return Err(eyre!("Resource address and value types don't match.")),
            }
        } else if let OptionalString::Some(expr) = &self.init_expr {
            expr.clone()
        } else {
            "".to_owned()
        }
        .trim()
        .to_owned();

        let expr = if let OptionalPath::Some(src) = &self.src {
            match res.fetch(&ResourceAddr::Text(src.clone()))? {
                ResourceValue::Text(expr) => (*expr).clone(),
                _ => return Err(eyre!("Resource address and value types don't match.")),
            }
        } else if let OptionalString::Some(expr) = &self.expr {
            expr.clone()
        } else {
            "".to_owned()
        }
        .trim()
        .to_owned();

        if expr.is_empty() {
            return Err(eyre!("Fn expression cannot be empty."));
        }

        let mut vars = self.vars.clone();
        vars.entry("self".to_owned()).or_insert(Value::Null);

        for (_, var) in self.in_mapping.iter() {
            if !vars.contains_key(var) {
                return Err(eyre!("Undefined variable ({var}) in `in_mapping`."));
            }
        }

        let evaluator = interpreter
            .parse(&init, &expr, &mut vars)
            .wrap_err("Failed to initialize function evaluator.")?;

        Ok(Box::new(StatefulFunction {
            done: false,
            name: self.name.clone(),
            vars,
            evaluator,
            on_start: self.on_start,
            on_change: self.on_change,
            once: self.once,
            in_mapping: self.in_mapping.clone(),
            in_update: self.in_update,
            lo_response: self.lo_response,
            out_result: self.out_result,
        }))
    }
}

impl StatefulAction for StatefulFunction {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        if self.once { DEFAULT } else { INFINITE }.into()
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
                    *entry = value.clone();
                }
            }
        }

        if self.on_start {
            if self.once && self.lo_response == 0 {
                self.done = true;
                sync_writer.push(SyncSignal::UpdateGraph);
            }

            self.eval(sync_writer, async_writer)
                .wrap_err("Failed to evaluate function.")
        } else {
            Ok(Signal::none())
        }
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        let mut news: Vec<(SignalId, Value)> = vec![];
        let mut changed = false;
        let mut updated = false;
        if let ActionSignal::StateChanged(_, signal) = signal {
            for id in signal {
                if let Some(var) = self.in_mapping.get(id) {
                    if let Some(entry) = self.vars.get_mut(var) {
                        *entry = state.get(id).unwrap().clone();
                    }
                    changed = true;
                }

                if *id == self.lo_response {
                    let result = state.get(id).unwrap();
                    self.vars.insert("self".to_owned(), result.clone());

                    if !self.name.is_empty() {
                        async_writer.push(LoggerSignal::Append(
                            "math".to_owned(),
                            (self.name.clone(), result.clone()),
                        ));
                    }

                    if self.out_result > 0 {
                        news.push((self.out_result, result.clone()));
                    }

                    if self.once {
                        self.done = true;
                        sync_writer.push(SyncSignal::UpdateGraph);
                    }
                }
            }

            if signal.contains(&self.in_update) {
                updated = true;
            }
        }

        if (changed && self.on_change) || updated {
            news.extend(
                self.eval(sync_writer, async_writer)
                    .wrap_err("Failed to evaluate function.")?,
            );
        }

        Ok(news.into())
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("name", format!("{:?}", self.name))])
            .collect()
    }
}

impl StatefulFunction {
    #[inline(always)]
    fn eval(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<Signal> {
        if self.lo_response > 0 {
            self.eval_lazy(sync_writer)
        } else {
            self.eval_blocking(async_writer)
        }
    }

    fn eval_blocking(&mut self, async_writer: &mut QWriter<AsyncSignal>) -> Result<Signal> {
        let result = self.evaluator.eval(&mut self.vars)?;

        self.vars.insert("self".to_owned(), result.clone());

        if !self.name.is_empty() {
            async_writer.push(LoggerSignal::Append(
                "math".to_owned(),
                (self.name.clone(), result.clone()),
            ));
        }

        if self.out_result > 0 {
            Ok(vec![(self.out_result, result)].into())
        } else {
            Ok(Signal::none())
        }
    }

    fn eval_lazy(&mut self, sync_writer: &mut QWriter<SyncSignal>) -> Result<Signal> {
        let loopback = {
            let signal_id = self.lo_response;
            let mut sync_writer = sync_writer.clone();

            Box::new(move |value: Value| {
                sync_writer.push(SyncSignal::Emit(
                    Instant::now(),
                    Signal::from(vec![(signal_id, value)]),
                ));
            })
        };

        let error = {
            let mut sync_writer = sync_writer.clone();

            Box::new(move |e: Error| {
                sync_writer.push(SyncSignal::Error(e));
            })
        };

        self.evaluator.eval_lazy(&mut self.vars, loopback, error)?;
        Ok(Signal::none())
    }
}
