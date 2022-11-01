use crate::resource::{LoopbackError, LoopbackResult, VarMap};
use cpython::{exc, FromPyObject, PyClone, PyDict, PyErr, PyNone, PyObject, PyResult, Python};
use eyre::{eyre, Context, Error, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::thread;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PyValue {
    None,
    Integer(i64),
    Float(f64),
    Bool(bool),
    String(String),
}

pub struct Evaluator {
    run: String,
    eval: String,
    vars: PyDict,
}

impl Evaluator {
    pub fn new(init: &str, expr: &str, vars: &mut VarMap) -> Result<Self> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        let dict = PyDict::new(py);
        for (name, value) in vars.iter() {
            match value {
                Value::Null => dict.set_item(py, name, PyNone),
                Value::Bool(v) => dict.set_item(py, name, v),
                Value::Integer(v) => dict.set_item(py, name, *v as i64),
                Value::Float(v) => dict.set_item(py, name, v),
                Value::Bytes(v) => dict.set_item(py, name, v),
                Value::Text(v) => dict.set_item(py, name, v),
                _ => return Err(eyre!("Cannot convert value ({value:?}) to python object.")),
            }
            .map_err(|e| eyre!("Failed to set variable ({name:?}) in python dict:\n{e:#?}"))?;
        }

        if !init.is_empty() {
            py.run(init, None, Some(&dict))
                .map_err(|e| eyre!("Failed to run python code:\n{e:#?}"))?;
        }

        let lines: Vec<_> = expr.trim().lines().collect();
        let run = lines[0..lines.len() - 1].join("\n");
        let eval = lines[lines.len() - 1].to_owned();

        Ok(Self {
            run,
            eval,
            vars: dict,
        })
    }

    pub fn eval(&self, vars: &mut VarMap) -> Result<Value> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        for (name, value) in vars.iter() {
            match value {
                Value::Null => self.vars.set_item(py, name, PyNone),
                Value::Bool(v) => self.vars.set_item(py, name, v),
                Value::Integer(v) => self.vars.set_item(py, name, *v as i64),
                Value::Float(v) => self.vars.set_item(py, name, v),
                Value::Bytes(v) => self.vars.set_item(py, name, v),
                Value::Text(v) => self.vars.set_item(py, name, v),
                _ => return Err(eyre!("Cannot convert value ({value:?}) to python object.")),
            }
            .map_err(|e| eyre!("Failed to set variable ({name:?}) in python dict:\n{e:#?}"))?;
        }

        if !self.run.is_empty() {
            py.run(&self.run, None, Some(&self.vars))
                .map_err(|e| eyre!("Failed to run python code:\n{e:#?}"))?;
        }

        let result: Value = py
            .eval(&self.eval, None, Some(&self.vars))
            .map_err(|e| eyre!("Failed to evaluate python expression:\n{e:#?}"))?
            .extract::<PyValue>(py)
            .map_err(|e| eyre!("Failed to extract python result:\n{e:#?}"))?
            .try_into()
            .wrap_err("Failed to convert python result to Value.")?;

        Ok(result)
    }

    pub fn eval_lazy(
        &self,
        vars: &mut VarMap,
        loopback: LoopbackResult,
        error: LoopbackError,
    ) -> Result<()> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        for (name, value) in vars.iter() {
            match value {
                Value::Null => self.vars.set_item(py, name, PyNone),
                Value::Bool(v) => self.vars.set_item(py, name, v),
                Value::Integer(v) => self.vars.set_item(py, name, *v as i64),
                Value::Float(v) => self.vars.set_item(py, name, v),
                Value::Bytes(v) => self.vars.set_item(py, name, v),
                Value::Text(v) => self.vars.set_item(py, name, v),
                _ => return Err(eyre!("Cannot convert value ({value:?}) to python object.")),
            }
            .map_err(|e| eyre!("Failed to set variable ({name:?}) in python dict:\n{e:#?}"))?;
        }

        let run = self.run.clone();
        let eval = self.eval.clone();
        let vars = self.vars.clone_ref(py);

        thread::spawn(move || {
            let gil = Python::acquire_gil();
            let py = gil.python();

            if !run.is_empty() {
                if let Err(e) = py.run(&run, None, Some(&vars)) {
                    error(eyre!("Failed to run python code:\n{e:#?}"));
                    return;
                }
            }

            let result = match py.eval(&eval, None, Some(&vars)) {
                Ok(r) => r,
                Err(e) => {
                    error(eyre!("Failed to evaluate python expression:\n{e:#?}"));
                    return;
                }
            };

            let result: PyValue = match result.extract(py) {
                Ok(r) => r,
                Err(e) => {
                    error(eyre!("Failed to extract python result:\n{e:#?}"));
                    return;
                }
            };

            let result = match result.try_into() {
                Ok(r) => r,
                Err(e) => {
                    error(eyre!("Failed to convert python result to Value:\n{e:#?}"));
                    return;
                }
            };

            loopback(result);
        });

        Ok(())
    }
}

impl<'a> FromPyObject<'a> for PyValue {
    fn extract(py: Python, obj: &PyObject) -> PyResult<Self> {
        if obj.is_none(py) {
            Ok(PyValue::None)
        } else if let Ok(v) = obj.extract::<i64>(py) {
            Ok(PyValue::Integer(v))
        } else if let Ok(v) = obj.extract::<f64>(py) {
            Ok(PyValue::Float(v))
        } else if let Ok(v) = obj.extract::<bool>(py) {
            Ok(PyValue::Bool(v))
        } else if let Ok(v) = obj.extract::<String>(py) {
            Ok(PyValue::String(v))
        } else {
            Err(PyErr::new::<exc::TypeError, _>(
                py,
                "Failed to convert PyObject to PyValue.",
            ))
        }
    }
}

impl From<PyValue> for Value {
    fn from(v: PyValue) -> Self {
        match v {
            PyValue::None => Value::Null,
            PyValue::Integer(v) => Value::Integer(v as i128),
            PyValue::Float(v) => Value::Float(v),
            PyValue::Bool(v) => Value::Bool(v),
            PyValue::String(v) => Value::Text(v),
        }
    }
}

impl TryFrom<Value> for PyValue {
    type Error = Error;

    fn try_from(v: Value) -> Result<Self> {
        match v {
            Value::Null => Ok(PyValue::None),
            Value::Bool(v) => Ok(PyValue::Bool(v)),
            Value::Integer(v) => Ok(PyValue::Integer(v as i64)),
            Value::Float(v) => Ok(PyValue::Float(v)),
            Value::Text(v) => Ok(PyValue::String(v)),
            _ => Err(eyre!("Failed to convert serde Value to PyValue.")),
        }
    }
}
