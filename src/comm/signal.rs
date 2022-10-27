use serde_cbor::Value;
use std::collections::btree_map::{IntoIter, Iter};
use std::collections::BTreeMap;

pub type SignalId = u16;

#[derive(Debug, Clone)]
pub struct Signal(BTreeMap<SignalId, Value>);

impl Signal {
    pub fn new(signals: impl Into<BTreeMap<SignalId, Value>>) -> Self {
        Self(signals.into())
    }

    pub fn none() -> Self {
        Self(BTreeMap::new())
    }

    pub fn get(&self, tag: SignalId) -> Option<&Value> {
        self.0.get(&tag)
    }

    pub fn iter(&self) -> Iter<SignalId, Value> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Vec<(SignalId, Value)>> for Signal {
    fn from(vec: Vec<(SignalId, Value)>) -> Self {
        Self(vec.into_iter().collect())
    }
}

impl IntoIterator for Signal {
    type Item = (SignalId, Value);
    type IntoIter = IntoIter<SignalId, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
