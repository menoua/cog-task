use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};

#[derive(Debug)]
pub struct Queue<T>(Arc<Mutex<VecDeque<T>>>);

#[derive(Debug)]
pub struct QReader<T>(Queue<T>, Sender<()>, Receiver<()>);

#[derive(Debug, Clone)]
pub struct QWriter<T>(Queue<T>, Sender<()>);

impl<T> Queue<T> {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(VecDeque::with_capacity(64))))
    }

    #[inline(always)]
    fn push(&mut self, msg: impl Into<T>) {
        self.0.lock().unwrap().push_back(msg.into());
    }

    #[inline(always)]
    fn pop(&mut self) -> Option<T> {
        self.0.lock().unwrap().pop_front()
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.0.lock().unwrap().clear();
    }
}

impl<T> Clone for Queue<T> {
    fn clone(&self) -> Self {
        Queue(self.0.clone())
    }
}

impl<T> QReader<T> {
    pub fn new() -> Self {
        let queue = Queue::new();
        let (tx, rx) = mpsc::channel();
        Self(queue, tx, rx)
    }

    #[inline(always)]
    pub fn push(&mut self, msg: impl Into<T>) {
        self.0.push(msg);
        self.1
            .send(())
            .expect("Signal buffer was dropped before the signal handle.");
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.2.recv().is_ok() {
            Some(self.0.pop().unwrap())
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn try_pop(&mut self) -> Option<T> {
        if let Ok(()) = self.2.try_recv() {
            Some(self.0.pop().unwrap())
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    #[inline(always)]
    pub fn writer(&self) -> QWriter<T> {
        QWriter(self.0.clone(), self.1.clone())
    }
}

impl<T> QWriter<T> {
    #[inline(always)]
    pub fn push(&mut self, msg: impl Into<T>) {
        self.0.push(msg);
        self.1
            .send(())
            .expect("Signal buffer was dropped before the signal handle.");
    }
}
