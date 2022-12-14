use eyre::{eyre, Result};
use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};

pub const QUEUE_SIZE: usize = 64;
pub const MAX_QUEUE_SIZE: usize = 256;

pub type Queue<T> = Arc<Mutex<VecDeque<T>>>;
pub struct QReader<T>(Queue<T>, Sender<()>, Receiver<()>);
pub struct QWriter<T>(Queue<T>, Sender<()>);

impl<T> QReader<T> {
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
            self.0.lock().unwrap().pop_front()
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn poll(&mut self) -> Result<Vec<T>>
    where
        T: Eq,
    {
        let mut signals = Vec::with_capacity(16);
        if self.2.recv().is_ok() {
            let mut queue = self.0.lock().unwrap();
            loop {
                let signal = queue.pop_front().unwrap();
                if !signals.contains(&signal) {
                    signals.push(signal);
                }
                if self.2.try_recv().is_err() {
                    break;
                }
                if signals.len() > MAX_QUEUE_SIZE {
                    return Err(eyre!("Signal queue exceeded MAX_QUEUE_SIZE."));
                }
            }

            Ok(signals)
        } else {
            Err(eyre!("Failed to poll. Ending sync queue."))
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

impl<T> Default for QReader<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> QWriter<T> {
    #[inline(always)]
    pub fn push(&mut self, msg: impl Into<T>) {
        let mut queue = self.0.lock().unwrap();
        if self.1.send(()).is_ok() {
            queue.push_back(msg.into());
        }
    }
}

impl<T> Clone for QWriter<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}
