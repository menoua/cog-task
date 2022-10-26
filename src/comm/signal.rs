use serde_cbor::Value;
use std::collections::hash_map::{IntoIter, Iter};
use std::collections::HashMap;

pub type SignalId = u16;

#[derive(Debug, Clone)]
pub struct Signal(HashMap<SignalId, Value>);

impl Signal {
    pub fn new(signals: HashMap<SignalId, Value>) -> Self {
        Self(signals)
    }

    pub fn get(&self, tag: SignalId) -> Option<&Value> {
        self.0.get(&tag)
    }

    pub fn iter(&self) -> Iter<SignalId, Value> {
        self.0.iter()
    }

    pub fn into_iter(self) -> IntoIter<SignalId, Value> {
        self.0.into_iter()
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
