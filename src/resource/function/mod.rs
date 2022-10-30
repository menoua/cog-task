use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::BTreeMap;

mod fasteval;
#[cfg(feature = "python")]
mod python;
#[cfg(feature = "savage")]
mod savage;

pub type VarMap = BTreeMap<String, Value>;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Interpreter {
    Inherit,
    Fasteval,
    #[cfg(feature = "savage")]
    Savage,
    #[cfg(feature = "python")]
    Python,
}

pub enum Evaluator {
    Fasteval(fasteval::Evaluator),
    #[cfg(feature = "savage")]
    Savage(savage::Evaluator),
    #[cfg(feature = "python")]
    Python(python::Evaluator),
}

impl Default for Interpreter {
    fn default() -> Self {
        Interpreter::Inherit
    }
}

impl Interpreter {
    pub fn parse(&self, init: &str, expr: &str, vars: &mut VarMap) -> Result<Evaluator> {
        match self {
            Interpreter::Inherit => Err(eyre!("Cannot parse with interpreter=`Inherit`.")),
            Interpreter::Fasteval => Ok(Evaluator::Fasteval(fasteval::Evaluator::new(
                init, expr, vars,
            )?)),
            #[cfg(feature = "savage")]
            Interpreter::Savage => Ok(Evaluator::Savage(savage::Evaluator::new(init, expr, vars)?)),
            #[cfg(feature = "python")]
            Interpreter::Python => Ok(Evaluator::Python(python::Evaluator::new(init, expr, vars)?)),
        }
    }

    pub fn or(&self, other: &Self) -> Self {
        if let Self::Inherit = self {
            *other
        } else {
            *self
        }
    }
}

impl Evaluator {
    pub fn eval(&self, vars: &mut VarMap) -> Result<Value> {
        match self {
            Evaluator::Fasteval(evaler) => evaler.eval(vars),
            #[cfg(feature = "savage")]
            Evaluator::Savage(evaler) => evaler.eval(vars),
            #[cfg(feature = "python")]
            Evaluator::Python(evaler) => evaler.eval(vars),
        }
    }
}
