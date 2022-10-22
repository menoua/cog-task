use crate::scheduler::processor::SyncSignal;
use serde_cbor::Value;
use std::collections::btree_map::Iter;
use std::collections::BTreeMap;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalSignal(BTreeMap<u16, Value>);

impl InternalSignal {
    pub fn new(signals: BTreeMap<u16, Value>) -> Self {
        Self(signals)
    }

    pub fn get(&self, tag: u16) -> Option<&Value> {
        self.0.get(&tag)
    }

    pub fn iter(&self) -> Iter<u16, Value> {
        self.0.iter()
    }
}

impl From<Vec<(u16, Value)>> for InternalSignal {
    fn from(vec: Vec<(u16, Value)>) -> Self {
        Self(vec.into_iter().collect())
    }
}

impl From<InternalSignal> for SyncSignal {
    fn from(signal: InternalSignal) -> Self {
        SyncSignal::Internal(Instant::now(), signal)
    }
}
