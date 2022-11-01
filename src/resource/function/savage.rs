use crate::resource::{LoopbackError, LoopbackResult, VarMap};
use eyre::{eyre, Result};
use num_traits::ToPrimitive;
use savage_core::expression::{Expression, Integer, Rational, RationalRepresentation};
use serde_cbor::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::thread;

static mut SAVAGE_EXPR: Option<Arc<Mutex<BTreeMap<usize, Expression>>>> = None;

#[derive(Clone)]
pub struct Evaluator(usize);

impl Evaluator {
    pub fn new(_init: &str, expr: &str, _vars: &mut VarMap) -> Result<Self> {
        let expression = expr
            .parse::<Expression>()
            .map_err(|e| eyre!("Failed to parse `savage` expression ({e:#?})."))?;

        let index = unsafe {
            if SAVAGE_EXPR.is_none() {
                SAVAGE_EXPR = Some(Arc::new(Mutex::new(BTreeMap::new())));
            }

            let mut global = SAVAGE_EXPR.as_ref().unwrap().lock().unwrap();
            let index = global.len();
            global.insert(index, expression);
            index
        };

        Ok(Self(index))
    }

    pub fn eval(&self, vars: &VarMap) -> Result<Value> {
        let expression = unsafe {
            if let Some(expr) = SAVAGE_EXPR.as_ref().unwrap().lock().unwrap().get(&self.0) {
                expr.clone()
            } else {
                return Err(eyre!("`savage::Expression` is missing."));
            }
        };

        let mut hash_vars = HashMap::new();
        for (s, v) in vars.iter() {
            let v = match v {
                Value::Null => Expression::Integer(Integer::from(0)),
                Value::Bool(v) => Expression::Boolean(*v),
                Value::Integer(v) => Expression::Integer(Integer::from(*v)),
                Value::Float(v) => Expression::Rational(
                    Rational::from_float(*v)
                        .ok_or_else(|| eyre!("Failed to convert float ({v}) to ratio."))?,
                    RationalRepresentation::Decimal,
                ),
                _ => Err(eyre!(
                    "Failed to convert variable ({s}) to savage Expression."
                ))?,
            };

            hash_vars.insert(s.clone(), v);
        }

        let result = expression
            .evaluate(hash_vars)
            .map_err(|e| eyre!("Failed to evaluate mathematical expression ({e:#?})."))?;

        match result {
            Expression::Integer(x) => {
                Ok(Value::Integer(x.to_i128().ok_or_else(|| {
                    eyre!("Failed to convert result to i128.")
                })?))
            }
            Expression::Rational(x, _) => Ok(Value::Float(
                x.numer()
                    .to_f64()
                    .ok_or_else(|| eyre!("Failed to convert numerator of result to f64."))?
                    / x.denom()
                        .to_f64()
                        .ok_or_else(|| eyre!("Failed to convert denominator of result to f64."))?,
            )),
            Expression::Complex(_, _) => Err(eyre!("Complex results are currently not supported.")),
            _ => Err(eyre!("`savage::Expression` is missing.")),
        }
    }

    pub fn eval_lazy(
        &self,
        vars: &VarMap,
        loopback: LoopbackResult,
        error: LoopbackError,
    ) -> Result<()> {
        let index = self.0;
        let vars = vars.clone();

        thread::spawn(move || {
            let mut hash_vars = HashMap::new();
            for (s, v) in vars.iter() {
                let v = match v {
                    Value::Null => Expression::Integer(Integer::from(0)),
                    Value::Bool(v) => Expression::Boolean(*v),
                    Value::Integer(v) => Expression::Integer(Integer::from(*v)),
                    Value::Float(v) => {
                        if let Some(v) = Rational::from_float(*v) {
                            Expression::Rational(v, RationalRepresentation::Decimal)
                        } else {
                            error(eyre!("Failed to convert float ({v}) to ratio."));
                            return;
                        }
                    }
                    _ => {
                        error(eyre!(
                            "Failed to convert variable ({s}) to savage Expression."
                        ));
                        return;
                    }
                };

                hash_vars.insert(s.clone(), v);
            }

            let result = unsafe {
                if let Some(expr) = SAVAGE_EXPR.as_ref().unwrap().lock().unwrap().get(&index) {
                    expr.evaluate(hash_vars)
                } else {
                    error(eyre!("Global `savage::Expression` map is missing."));
                    return;
                }
            };

            match result {
                Ok(Expression::Integer(x)) => {
                    let r = x
                        .to_i128()
                        .ok_or_else(|| eyre!("Failed to convert result to i128."));

                    match r {
                        Ok(r) => loopback(Value::Integer(r)),
                        Err(e) => error(e),
                    }
                }
                Ok(Expression::Rational(x, _)) => {
                    let num = x
                        .numer()
                        .to_f64()
                        .ok_or_else(|| eyre!("Failed to convert numerator of result to f64."));
                    let den = x
                        .denom()
                        .to_f64()
                        .ok_or_else(|| eyre!("Failed to convert denominator of result to f64."));

                    match (num, den) {
                        (Ok(num), Ok(den)) => loopback(Value::Float(num / den)),
                        (Err(e), _) => error(e),
                        (_, Err(e)) => error(e),
                    }
                }
                Ok(Expression::Complex(_, _)) => {
                    error(eyre!("Complex results are currently not supported."))
                }
                Err(e) => error(eyre!(
                    "Failed to evaluate mathematical expression ({e:#?})."
                )),
                _ => error(eyre!("`savage::Expression` is missing.")),
            }
        });

        Ok(())
    }
}

impl Drop for Evaluator {
    fn drop(&mut self) {
        unsafe {
            SAVAGE_EXPR
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .remove(&self.0);
        }
    }
}
