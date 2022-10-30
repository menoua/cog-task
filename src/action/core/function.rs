use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{
    Evaluator, Interpreter, IoManager, LoggerSignal, ResourceAddr, ResourceManager, ResourceValue,
};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eyre::{eyre, Context, Error, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Function {
    #[serde(default)]
    name: String,
    #[serde(default)]
    expr: String,
    #[serde(default)]
    src: PathBuf,
    #[serde(default)]
    init_expr: String,
    #[serde(default)]
    init_src: PathBuf,
    #[serde(default)]
    vars: BTreeMap<String, Value>,
    #[serde(default)]
    interpreter: Interpreter,
    #[serde(default = "defaults::on_start")]
    on_start: bool,
    #[serde(default = "defaults::on_change")]
    on_change: bool,
    #[serde(default)]
    persistent: bool,
    #[serde(default)]
    in_mapping: BTreeMap<SignalId, String>,
    #[serde(default)]
    in_update: SignalId,
    #[serde(default)]
    out_result: SignalId,
}

stateful!(Function {
    name: String,
    vars: BTreeMap<String, Value>,
    evaluator: Evaluator,
    on_start: bool,
    on_change: bool,
    persistent: bool,
    in_mapping: BTreeMap<SignalId, String>,
    in_update: SignalId,
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
        let has_expr = !self.expr.is_empty();
        let has_src = !self.src.as_os_str().is_empty();
        match (has_expr, has_src) {
            (false, false) => Err(eyre!("`expr` and `src` cannot both be empty."))?,
            (true, true) => Err(eyre!("Only one of `expr` and `src` should be set."))?,
            _ => {}
        };

        let has_init_expr = !self.init_expr.is_empty();
        let has_init_src = !self.init_src.as_os_str().is_empty();
        if has_init_expr && has_init_src {
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
        signals.insert(self.in_update);
        signals
    }

    #[inline]
    fn out_signals(&self) -> BTreeSet<SignalId> {
        BTreeSet::from([self.out_result])
    }

    fn resources(&self, _config: &Config) -> Vec<ResourceAddr> {
        let mut resources = vec![];
        if !self.src.as_os_str().is_empty() {
            resources.push(ResourceAddr::Text(self.src.clone()));
        }
        if !self.init_src.as_os_str().is_empty() {
            resources.push(ResourceAddr::Text(self.init_src.clone()));
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

        let init = if !self.init_src.as_os_str().is_empty() {
            match res.fetch(&ResourceAddr::Text(self.init_src.clone()))? {
                ResourceValue::Text(expr) => (*expr).clone(),
                _ => return Err(eyre!("Resource address and value types don't match.")),
            }
        } else {
            self.init_expr.clone()
        }
        .trim()
        .to_owned();

        let expr = if !self.src.as_os_str().is_empty() {
            match res.fetch(&ResourceAddr::Text(self.src.clone()))? {
                ResourceValue::Text(expr) => (*expr).clone(),
                _ => return Err(eyre!("Resource address and value types don't match.")),
            }
        } else {
            self.expr.clone()
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
            persistent: self.persistent,
            in_mapping: self.in_mapping.clone(),
            in_update: self.in_update,
            out_result: self.out_result,
        }))
    }
}

impl StatefulAction for StatefulFunction {
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
                    *entry = value.clone();
                }
            }
        }

        if !self.persistent {
            self.done = true;
            sync_writer.push(SyncSignal::UpdateGraph);
        }

        if self.on_start {
            self.eval(async_writer)
                .wrap_err("Failed to evaluate function.")
        } else {
            Ok(Signal::none())
        }
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
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
            }

            if signal.contains(&self.in_update) {
                updated = true;
            }
        }

        if (changed && self.on_change) || updated {
            self.eval(async_writer)
                .wrap_err("Failed to evaluate function.")
        } else {
            Ok(Signal::none())
        }
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("name", format!("{:?}", self.name))])
            .collect()
    }
}

impl StatefulFunction {
    fn eval(&mut self, async_writer: &mut QWriter<AsyncSignal>) -> Result<Signal> {
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
}
