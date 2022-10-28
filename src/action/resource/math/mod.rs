use eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod fasteval;
#[cfg(feature = "savage")]
mod savage;

pub type VarMap = BTreeMap<String, f64>;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Interpreter {
    Fasteval,
    #[cfg(feature = "savage")]
    Savage,
}

pub enum Evaluator {
    Fasteval(fasteval::Evaluator),
    #[cfg(feature = "savage")]
    Savage(savage::Evaluator),
}

impl Default for Interpreter {
    fn default() -> Self {
        Interpreter::Fasteval
    }
}

impl Interpreter {
    pub fn parse(&self, expr: &str) -> Result<Evaluator> {
        match self {
            Interpreter::Fasteval => Ok(Evaluator::Fasteval(fasteval::Evaluator::new(expr)?)),
            #[cfg(feature = "savage")]
            Interpreter::Savage => Ok(Evaluator::Savage(savage::Evaluator::new(expr)?)),
        }
    }
}

impl Evaluator {
    pub fn eval(&self, vars: &mut VarMap) -> Result<f64> {
        match self {
            Evaluator::Fasteval(evaler) => evaler.eval(vars),
            #[cfg(feature = "savage")]
            Evaluator::Savage(evaler) => evaler.eval(vars),
        }
    }
}
