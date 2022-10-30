use crate::resource::VarMap;
use eyre::{eyre, Result};
use num_traits::ToPrimitive;
use savage_core::expression::{Expression, Integer, Rational, RationalRepresentation};
use serde_cbor::Value;
use std::collections::{BTreeMap, HashMap};

static mut SAVAGE_EXPR: Option<BTreeMap<usize, Expression>> = None;

pub struct Evaluator(usize);

impl Evaluator {
    pub fn new(_init: &str, expr: &str, _vars: &mut VarMap) -> Result<Self> {
        let expression = expr
            .parse::<Expression>()
            .map_err(|e| eyre!("Failed to parse `savage` expression ({e:?})."))?;

        let index = unsafe {
            if SAVAGE_EXPR.is_none() {
                SAVAGE_EXPR = Some(BTreeMap::new());
            }

            let map = SAVAGE_EXPR.as_mut().unwrap();
            let index = map.len();
            map.insert(index, expression);
            index
        };

        Ok(Self(index))
    }

    pub fn eval(&self, vars: &VarMap) -> Result<Value> {
        let expression = unsafe {
            if let Some(map) = SAVAGE_EXPR.as_mut() {
                map.get(&self.0)
            } else {
                None
            }
        };

        let expression = if let Some(expr) = expression {
            expr
        } else {
            return Err(eyre!("`savage::Expression` is missing."));
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
            .map_err(|e| eyre!("Failed to evaluate mathematical expression ({e:?})."))?;

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
}

impl Drop for Evaluator {
    fn drop(&mut self) {
        unsafe {
            if let Some(map) = SAVAGE_EXPR.as_mut() {
                map.remove(&self.0);
            }
        }
    }
}
