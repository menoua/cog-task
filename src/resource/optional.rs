use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OptionalInt {
    Some(i64),
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OptionalUInt {
    Some(u64),
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OptionalFloat {
    Some(f64),
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OptionalString {
    Some(String),
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OptionalPath {
    Some(PathBuf),
    None,
}

impl Default for OptionalInt {
    fn default() -> Self {
        Self::None
    }
}

impl OptionalInt {
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl Default for OptionalUInt {
    fn default() -> Self {
        Self::None
    }
}

impl OptionalUInt {
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl Default for OptionalFloat {
    fn default() -> Self {
        Self::None
    }
}

impl From<Option<f32>> for OptionalFloat {
    fn from(opt: Option<f32>) -> Self {
        match opt {
            None => Self::None,
            Some(f) => Self::Some(f as f64),
        }
    }
}

impl From<Option<f64>> for OptionalFloat {
    fn from(opt: Option<f64>) -> Self {
        match opt {
            None => Self::None,
            Some(f) => Self::Some(f),
        }
    }
}

impl OptionalFloat {
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn as_f32(&self) -> Option<f32> {
        match self {
            OptionalFloat::Some(f) => Some(*f as f32),
            OptionalFloat::None => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            OptionalFloat::Some(f) => Some(*f),
            OptionalFloat::None => None,
        }
    }
}

impl Default for OptionalString {
    fn default() -> Self {
        Self::None
    }
}

impl OptionalString {
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl Default for OptionalPath {
    fn default() -> Self {
        Self::None
    }
}

impl OptionalPath {
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}
