use crate::scheduler::processor::SyncSignal;
use crate::scheduler::State;
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::hash_map::Iter;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub enum SignalId {
    None,
    #[serde(rename = "i")]
    Internal(u16),
    #[serde(rename = "e")]
    External(u16),
    #[serde(rename = "s")]
    State(u16),
}

impl Default for SignalId {
    fn default() -> Self {
        SignalId::None
    }
}

impl SignalId {
    pub fn is_none(&self) -> bool {
        matches!(self, SignalId::None)
    }
}

#[derive(Debug, Clone)]
pub struct Signal(HashMap<SignalId, Value>);

pub type IntSignal = HashMap<u16, Value>;
pub type ExtSignal = HashMap<u16, Value>;

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

    pub fn split(self) -> (IntSignal, ExtSignal, State) {
        let mut ints = HashMap::new();
        let mut exts = HashMap::new();
        let mut state = HashMap::new();
        for (k, v) in self.0.into_iter() {
            match k {
                SignalId::None => None,
                SignalId::Internal(i) => ints.insert(i, v),
                SignalId::External(i) => exts.insert(i, v),
                SignalId::State(i) => state.insert(i, v),
            };
        }

        (ints, exts, state)
    }
}

impl From<Vec<(SignalId, Value)>> for Signal {
    fn from(vec: Vec<(SignalId, Value)>) -> Self {
        Self(vec.into_iter().collect())
    }
}

impl From<Signal> for SyncSignal {
    fn from(signal: Signal) -> Self {
        SyncSignal::Emit(Instant::now(), signal)
    }
}
