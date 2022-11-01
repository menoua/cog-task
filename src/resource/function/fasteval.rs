use crate::resource::{LoopbackError, LoopbackResult, VarMap};
use eyre::{eyre, Context, Result};
use fasteval::{Compiler, Evaler};
use serde_cbor::Value;
use std::collections::BTreeMap;

pub struct Evaluator {
    _parser: Box<fasteval::Parser>,
    slab: Box<fasteval::Slab>,
    instr: Box<fasteval::Instruction>,
}

impl Evaluator {
    pub fn new(_init: &str, expr: &str, _vars: &mut VarMap) -> Result<Self> {
        let parser = fasteval::Parser::new();
        let mut slab = fasteval::Slab::new();
        let instr = parser
            .parse(expr, &mut slab.ps)
            .wrap_err("Failed to parse math expression with fasteval.")?
            .from(&slab.ps)
            .compile(&slab.ps, &mut slab.cs);

        Ok(Self {
            _parser: Box::new(parser),
            slab: Box::new(slab),
            instr: Box::new(instr),
        })
    }

    pub fn eval(&self, vars: &mut VarMap) -> Result<Value> {
        use fasteval::eval_compiled_ref;

        let mut vars: BTreeMap<String, f64> = vars
            .iter()
            .map(|(name, value)| {
                let value = match value {
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

                (name.clone(), value)
            })
            .collect();

        Ok(Value::Float(eval_compiled_ref!(
            self.instr.as_ref(),
            self.slab.as_ref(),
            &mut vars
        )))
    }

    pub fn eval_lazy(
        &self,
        _vars: &mut VarMap,
        _loopback: LoopbackResult,
        _error: LoopbackError,
    ) -> Result<()> {
        Err(eyre!(
            "Fasteval interpreter does not support non-blocking execution."
        ))
    }
}
