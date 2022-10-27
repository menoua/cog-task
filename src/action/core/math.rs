use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT, INFINITE};
use crate::comm::{QWriter, SignalId};
use crate::resource::ResourceMap;
use crate::server::{AsyncSignal, Config, LoggerSignal, State, SyncSignal, IO};
use eyre::{eyre, Error, Result};
use fasteval::{Compiler, Evaler};
use num_traits::ToPrimitive;
use regex::Regex;
use savage_core::expression as savage;
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

static mut SAVAGE_EXPR: Option<BTreeMap<usize, savage::Expression>> = None;

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
    parser: Parser,
    persistent: bool,
    in_mapping: BTreeMap<SignalId, String>,
    in_update: SignalId,
    out_result: SignalId,
});

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Interpreter {
    Fasteval,
    Savage,
}

impl Default for Interpreter {
    fn default() -> Self {
        Interpreter::Fasteval
    }
}

enum Parser {
    Fasteval(
        Box<fasteval::Parser>,
        Box<fasteval::Slab>,
        Box<fasteval::Instruction>,
    ),
    Savage(usize),
}

impl Action for Math {
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
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Ok(Box::new(StatefulMath {
            done: false,
            _expr: self.expr.clone(),
            name: self.name.clone(),
            vars: self.vars.clone(),
            parser: self.get_parser()?,
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
    ) -> Result<(), Error> {
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

        let result = self.eval()?;

        if self.out_result > 0 {
            sync_writer.push(SyncSignal::Emit(
                Instant::now(),
                vec![(self.out_result, Value::Float(result))].into(),
            ));
        }

        if !self.name.is_empty() {
            async_writer.push(LoggerSignal::Append(
                "math".to_owned(),
                (self.name.clone(), Value::Float(result)),
            ));
        }

        if !self.persistent {
            self.done = true;
            sync_writer.push(SyncSignal::UpdateGraph);
        }

        Ok(())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Vec<SyncSignal>> {
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
            return Ok(vec![]);
        }

        let result = self.eval()?;

        if self.out_result > 0 {
            sync_writer.push(SyncSignal::Emit(
                Instant::now(),
                vec![(self.out_result, Value::Float(result))].into(),
            ));
        }

        if !self.name.is_empty() {
            async_writer.push(LoggerSignal::Append(
                "math".to_owned(),
                (self.name.clone(), Value::Float(result)),
            ));
        }

        Ok(vec![])
    }

    #[inline]
    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<()> {
        self.drop_parser();
        self.done = true;
        Ok(())
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("name", format!("{:?}", self.name))])
            .collect()
    }
}

impl Math {
    fn get_parser(&self) -> Result<Parser> {
        match self.interpreter {
            Interpreter::Fasteval => {
                let parser = fasteval::Parser::new();
                let mut slab = fasteval::Slab::new();
                let instr = parser
                    .parse(&self.expr, &mut slab.ps)?
                    .from(&slab.ps)
                    .compile(&slab.ps, &mut slab.cs);

                Ok(Parser::Fasteval(
                    Box::new(parser),
                    Box::new(slab),
                    Box::new(instr),
                ))
            }
            Interpreter::Savage => {
                let expression = self
                    .expr
                    .parse::<savage::Expression>()
                    .map_err(|e| eyre!("Failed to parse `savage` expression ({e:?})."))?;

                Ok(Parser::Savage(savage_ext::new(expression)))
            }
        }
    }
}

impl StatefulMath {
    fn eval(&mut self) -> Result<f64> {
        let result = match &self.parser {
            Parser::Fasteval(_, slab, instr) => {
                use fasteval::eval_compiled_ref;

                Ok(eval_compiled_ref!(
                    instr.as_ref(),
                    slab.as_ref(),
                    &mut self.vars
                ))
            }
            Parser::Savage(parser_id) => {
                use savage::{Expression, Rational, RationalRepresentation};

                if let Some(expression) = savage_ext::get(parser_id) {
                    let mut vars = HashMap::new();
                    for (s, f) in self.vars.iter() {
                        vars.insert(
                            s.clone(),
                            Expression::Rational(
                                Rational::from_float(*f).ok_or_else(|| {
                                    eyre!("Failed to convert float ({f}) to ratio.")
                                })?,
                                RationalRepresentation::Decimal,
                            ),
                        );
                    }
                    vars.insert("self".to_owned(), self.value);

                    let result = expression.evaluate(vars).map_err(|e| {
                        eyre!("Failed to evaluate mathematical expression ({e:?}).")
                    })?;

                    match result {
                        Expression::Integer(x) => x
                            .to_f64()
                            .ok_or_else(|| eyre!("Failed to convert integer result to f64.")),
                        Expression::Rational(x, _) => Ok(x.numer().to_f64().ok_or_else(|| {
                            eyre!("Failed to convert numerator of result to f64.")
                        })? / x.denom().to_f64().ok_or_else(
                            || eyre!("Failed to convert denominator of result to f64."),
                        )?),
                        Expression::Complex(_, _) => {
                            Err(eyre!("Complex results are currently not supported."))
                        }
                        _ => Err(eyre!("`savage::Expression` is missing.")),
                    }
                } else {
                    Err(eyre!("`savage::Expression` is missing."))
                }
            }
        }?;

        self.vars.insert("self".to_owned(), result);
        Ok(result)
    }

    fn drop_parser(&mut self) {
        if let Parser::Savage(parser_id) = &mut self.parser {
            savage_ext::drop(*parser_id);
        }
    }
}

mod savage_ext {
    use super::SAVAGE_EXPR;
    use savage_core::expression::Expression;
    use std::collections::BTreeMap;

    pub fn new(expression: Expression) -> usize {
        unsafe {
            if SAVAGE_EXPR.is_none() {
                SAVAGE_EXPR = Some(BTreeMap::new());
            }

            let map = SAVAGE_EXPR.as_mut().unwrap();
            let index = map.len();
            map.insert(index, expression);
            index
        }
    }

    pub fn get(index: &usize) -> Option<&Expression> {
        unsafe {
            if let Some(map) = SAVAGE_EXPR.as_mut() {
                map.get(index)
            } else {
                None
            }
        }
    }

    pub fn drop(index: usize) {
        unsafe {
            if let Some(map) = SAVAGE_EXPR.as_mut() {
                map.remove(&index);
            }
        }
    }

    #[allow(unused)]
    pub fn drop_all() {
        unsafe {
            SAVAGE_EXPR = None;
        }
    }
}
