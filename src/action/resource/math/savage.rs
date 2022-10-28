use crate::resource::VarMap;
use eyre::{eyre, Result};
use num_traits::ToPrimitive;
use savage_core::expression::{Expression, Rational, RationalRepresentation};
use std::collections::{BTreeMap, HashMap};

static mut SAVAGE_EXPR: Option<BTreeMap<usize, Expression>> = None;

pub struct Evaluator(usize);

impl Evaluator {
    pub fn new(expr: &str) -> Result<Self> {
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

    pub fn eval(&self, vars: &VarMap) -> Result<f64> {
        let expression = unsafe {
            if let Some(map) = SAVAGE_EXPR.as_mut() {
                map.get(&self.0)
            } else {
                None
            }
        };

        if let Some(expression) = expression {
            let mut hash_vars = HashMap::new();
            for (s, f) in vars.iter() {
                hash_vars.insert(
                    s.clone(),
                    Expression::Rational(
                        Rational::from_float(*f)
                            .ok_or_else(|| eyre!("Failed to convert float ({f}) to ratio."))?,
                        RationalRepresentation::Decimal,
                    ),
                );
            }

            let result = expression
                .evaluate(hash_vars)
                .map_err(|e| eyre!("Failed to evaluate mathematical expression ({e:?})."))?;

            match result {
                Expression::Integer(x) => x
                    .to_f64()
                    .ok_or_else(|| eyre!("Failed to convert integer result to f64.")),
                Expression::Rational(x, _) => Ok(x
                    .numer()
                    .to_f64()
                    .ok_or_else(|| eyre!("Failed to convert numerator of result to f64."))?
                    / x.denom()
                        .to_f64()
                        .ok_or_else(|| eyre!("Failed to convert denominator of result to f64."))?),
                Expression::Complex(_, _) => {
                    Err(eyre!("Complex results are currently not supported."))
                }
                _ => Err(eyre!("`savage::Expression` is missing.")),
            }
        } else {
            Err(eyre!("`savage::Expression` is missing."))
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
