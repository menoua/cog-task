use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

impl From<Option<i64>> for OptionalInt {
    fn from(opt: Option<i64>) -> Self {
        match opt {
            None => Self::None,
            Some(v) => Self::Some(v),
        }
    }
}

impl OptionalInt {
    pub fn as_ref(&self) -> Option<&i64> {
        match self {
            OptionalInt::Some(v) => Some(v),
            OptionalInt::None => None,
        }
    }
}

impl Default for OptionalUInt {
    fn default() -> Self {
        Self::None
    }
}

impl From<Option<u64>> for OptionalUInt {
    fn from(opt: Option<u64>) -> Self {
        match opt {
            None => Self::None,
            Some(v) => Self::Some(v),
        }
    }
}

impl OptionalUInt {
    pub fn as_ref(&self) -> Option<&u64> {
        match self {
            OptionalUInt::Some(v) => Some(v),
            OptionalUInt::None => None,
        }
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
            Some(v) => Self::Some(v as f64),
        }
    }
}

impl From<Option<f64>> for OptionalFloat {
    fn from(opt: Option<f64>) -> Self {
        match opt {
            None => Self::None,
            Some(v) => Self::Some(v),
        }
    }
}

impl OptionalFloat {
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            OptionalFloat::Some(v) => Some(*v as f32),
            OptionalFloat::None => None,
        }
    }

    pub fn as_ref(&self) -> Option<&f64> {
        match self {
            OptionalFloat::Some(v) => Some(v),
            OptionalFloat::None => None,
        }
    }
}

impl Default for OptionalString {
    fn default() -> Self {
        Self::None
    }
}

impl From<Option<String>> for OptionalString {
    fn from(opt: Option<String>) -> Self {
        match opt {
            None => Self::None,
            Some(v) => Self::Some(v),
        }
    }
}

impl OptionalString {
    pub fn as_ref(&self) -> Option<&str> {
        match self {
            OptionalString::Some(v) => Some(v),
            OptionalString::None => None,
        }
    }
}

impl Default for OptionalPath {
    fn default() -> Self {
        Self::None
    }
}

impl From<Option<PathBuf>> for OptionalPath {
    fn from(opt: Option<PathBuf>) -> Self {
        match opt {
            None => Self::None,
            Some(v) => Self::Some(v),
        }
    }
}

impl OptionalPath {
    pub fn as_ref(&self) -> Option<&Path> {
        match self {
            OptionalPath::Some(v) => Some(v),
            OptionalPath::None => None,
        }
    }
}
