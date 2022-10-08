use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub type Destination = Vec<u32>;

#[derive(Debug, Clone)]
pub struct MessageBuffer<T>(
    Arc<Mutex<VecDeque<(Destination, T)>>>,
    Arc<Mutex<VecDeque<(Destination, T)>>>,
);

impl<T> MessageBuffer<T> {
    pub fn new() -> Self {
        Self(
            Arc::new(Mutex::new(VecDeque::with_capacity(64))),
            Arc::new(Mutex::new(VecDeque::with_capacity(64))),
        )
    }

    #[inline(always)]
    pub fn push_sync(&mut self, dst: Destination, msg: T) {
        self.0.lock().unwrap().push_back((dst, msg));
    }

    #[inline(always)]
    pub fn pop_sync(&mut self) -> Option<(Destination, T)> {
        self.0.lock().unwrap().pop_front()
    }

    #[inline(always)]
    pub fn clear_sync(&mut self) {
        self.0.lock().unwrap().clear();
    }

    #[inline(always)]
    pub fn push_async(&mut self, dst: Destination, msg: T) {
        self.1.lock().unwrap().push_back((dst, msg));
    }

    #[inline(always)]
    pub fn pop_async(&mut self) -> Option<(Destination, T)> {
        self.1.lock().unwrap().pop_front()
    }

    #[inline(always)]
    pub fn clear_async(&mut self) {
        self.1.lock().unwrap().clear();
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.clear_sync();
        self.clear_async();
    }
}
