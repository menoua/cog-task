use crate::scheduler::processor::SyncSignal;
use ron::Value;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalSignal(Vec<(String, Value)>);

pub struct IntSignalIterator<'a> {
    signal: &'a InternalSignal,
    index: usize,
}

impl InternalSignal {
    pub fn new(content: Vec<(String, Value)>) -> Self {
        Self(content)
    }
}

impl From<InternalSignal> for SyncSignal {
    fn from(signal: InternalSignal) -> Self {
        SyncSignal::Internal(Instant::now(), signal)
    }
}

impl<'a> Iterator for IntSignalIterator<'a> {
    type Item = (&'a str, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.signal.0.len() {
            return None;
        }

        let (name, value) = &self.signal.0[self.index];
        self.index += 1;
        Some((name.as_str(), value))
    }
}

impl<'a> IntoIterator for &'a InternalSignal {
    type Item = (&'a str, &'a Value);
    type IntoIter = IntSignalIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        IntSignalIterator {
            signal: self,
            index: 0,
        }
    }
}
