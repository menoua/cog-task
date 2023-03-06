use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{IoManager, OptionalUInt, ResourceAddr, ResourceManager};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eframe::egui::{Response, Ui};
use eyre::{eyre, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};
use std::sync::mpsc::{self, RecvError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Deserialize, Serialize)]
pub struct Repeat {
    inner: Box<dyn Action>,
    #[serde(default)]
    iters: OptionalUInt,
    #[serde(default = "defaults::prefetch")]
    prefetch: u64,
}

stateful!(Repeat {
    inner: Box<dyn StatefulAction>,
    iters: Option<u64>,
    queue: Arc<Mutex<VecDeque<Box<dyn StatefulAction>>>>,
    link: Sender<()>,
});

mod defaults {
    pub fn prefetch() -> u64 {
        3
    }
}

impl Action for Repeat {
    fn in_signals(&self) -> BTreeSet<SignalId> {
        self.inner.in_signals()
    }

    fn out_signals(&self) -> BTreeSet<SignalId> {
        self.inner.out_signals()
    }

    fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
        self.inner.resources(config)
    }

    fn stateful(
        &self,
        io: &IoManager,
        res: &ResourceManager,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        let (tx, rx) = mpsc::channel();

        let iters = if let OptionalUInt::Some(n) = self.iters {
            Some(n)
        } else {
            None
        };
        let prefetch = if let Some(n) = iters {
            self.prefetch.min(n)
        } else {
            self.prefetch
        };

        let mut queue = VecDeque::with_capacity(prefetch as usize);
        for _ in 0..prefetch {
            queue.push_back(
                self.inner
                    .stateful(io, res, config, sync_writer, async_writer)?,
            );
        }

        let queue = Arc::new(Mutex::new(queue));

        {
            let queue = queue.clone();
            let blueprint = serde_cbor::to_vec(&self.inner)
                .wrap_err("Failed to serialize action blueprint.")?;

            let res = res.clone();
            let config = config.clone();
            let mut sync_writer = sync_writer.clone();
            let async_writer = async_writer.clone();

            thread::spawn(move || {
                let io = match IoManager::new(&config)
                    .wrap_err("Failed to create new IoManager for prefetcher.")
                {
                    Ok(io) => io,
                    Err(e) => {
                        sync_writer.push(SyncSignal::Error(e));
                        return;
                    }
                };

                let blueprint: Box<dyn Action> = match serde_cbor::from_slice(&blueprint)
                    .wrap_err("Failed to deserialize action blueprint.")
                {
                    Ok(v) => v,
                    Err(e) => {
                        sync_writer.push(SyncSignal::Error(e));
                        return;
                    }
                };

                loop {
                    if let Err(RecvError) = rx.recv() {
                        break;
                    } else {
                        match blueprint
                            .stateful(&io, &res, &config, &sync_writer, &async_writer)
                            .wrap_err("Failed to prefetch inner stateful action for Repeat.")
                        {
                            Ok(inner) => {
                                queue.lock().unwrap().push_back(inner);
                            }
                            Err(e) => {
                                sync_writer.push(SyncSignal::Error(e));
                                break;
                            }
                        }
                    }
                }
            });
        }

        Ok(Box::new(StatefulRepeat {
            done: false,
            inner: self
                .inner
                .stateful(io, res, config, sync_writer, async_writer)?,
            iters,
            queue,
            link: tx,
        }))
    }
}

impl StatefulAction for StatefulRepeat {
    impl_stateful!();

    fn props(&self) -> Props {
        if self.iters.is_none() {
            (self.inner.props().bits() | INFINITE).into()
        } else {
            self.inner.props()
        }
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        if let Some(0) = self.iters {
            self.done = true;
            sync_writer.push(SyncSignal::UpdateGraph);
            Ok(Signal::none())
        } else {
            self.iters = self.iters.map(|n| n - 1);
            self.inner.start(sync_writer, async_writer, state)
        }
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        let mut news = vec![];
        news.extend(
            self.inner
                .update(signal, sync_writer, async_writer, state)?,
        );

        if self.inner.is_over()? {
            if let Some(0) = self.iters {
                self.done = true;
                sync_writer.push(SyncSignal::UpdateGraph);
                return Ok(news.into());
            }

            if let Some(inner) = self.queue.lock().unwrap().pop_front() {
                self.inner = inner;
                news.extend(self.inner.start(sync_writer, async_writer, state)?);
            } else {
                return Err(eyre!(
                    "Failed to immediately restart action. Try increasing prefetch queue size."
                ));
            }

            if self.link.send(()).is_err() {
                return Err(eyre!("Action prefetcher did not respond to request."));
            }

            self.iters = self.iters.map(|n| n - 1);
        }

        Ok(news.into())
    }

    fn show(
        &mut self,
        ui: &mut Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Response> {
        self.inner.show(ui, sync_writer, async_writer, state)
    }

    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        self.inner.stop(sync_writer, async_writer, state)
    }
}
