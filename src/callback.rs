use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub type Destination = Vec<u32>;

#[derive(Debug, Clone)]
pub struct CallbackQueue<T>(Arc<Mutex<VecDeque<(Destination, T)>>>);

impl<T> CallbackQueue<T> {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(VecDeque::with_capacity(64))))
    }

    #[inline(always)]
    pub fn push(&mut self, dst: Destination, msg: impl Into<T>) {
        self.0.lock().unwrap().push_back((dst, msg.into()));
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<(Destination, T)> {
        self.0.lock().unwrap().pop_front()
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.0.lock().unwrap().clear();
    }
}
