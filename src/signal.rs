use std::collections::VecDeque;
use std::fmt::Debug;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};

pub const QUEUE_SIZE: usize = 64;
pub type Queue<T> = Arc<Mutex<VecDeque<T>>>;

#[derive(Debug)]
pub struct QReader<T: Debug>(Queue<T>, Sender<()>, Receiver<()>);

#[derive(Debug, Clone)]
pub struct QWriter<T: Debug>(Queue<T>, Sender<()>);

impl<T: Debug> QReader<T> {
    pub fn new() -> Self {
        let queue = Arc::new(Mutex::new(VecDeque::with_capacity(QUEUE_SIZE)));
        let (tx, rx) = mpsc::channel();
        Self(queue, tx, rx)
    }

    #[inline(always)]
    pub fn push(&mut self, msg: impl Into<T>) {
        let mut queue = self.0.lock().unwrap();
        if self.1.send(()).is_ok() {
            queue.push_back(msg.into());
        }
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.2.recv().is_ok() {
            Some(self.0.lock().unwrap().pop_front().unwrap())
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn try_pop(&mut self) -> Option<T> {
        if let Ok(()) = self.2.try_recv() {
            Some(self.0.lock().unwrap().pop_front().unwrap())
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.0.lock().unwrap().clear();
    }

    #[inline(always)]
    pub fn writer(&self) -> QWriter<T> {
        QWriter(self.0.clone(), self.1.clone())
    }
}

impl<T: Debug> QWriter<T> {
    #[inline(always)]
    pub fn push(&mut self, msg: impl Into<T>) {
        let mut queue = self.0.lock().unwrap();
        if self.1.send(()).is_ok() {
            queue.push_back(msg.into());
        }
    }
}
