use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::comm::{QWriter, Signal, SignalId};
use crate::resource::{IoManager, ResourceAddr, ResourceManager};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use eframe::egui::{Response, Ui};
use eyre::{eyre, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};
use std::sync::mpsc::{self, RecvError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Deserialize, Serialize)]
pub struct Repeat(
    Box<dyn Action>,
    #[serde(default = "defaults::prefetch")] usize,
);

stateful!(Repeat {
    inner: Box<dyn StatefulAction>,
    queue: Arc<Mutex<VecDeque<Box<dyn StatefulAction>>>>,
    link: Sender<()>,
});

mod defaults {
    pub fn prefetch() -> usize {
        2
    }
}

impl Action for Repeat {
    fn in_signals(&self) -> BTreeSet<SignalId> {
        self.0.in_signals()
    }

    fn out_signals(&self) -> BTreeSet<SignalId> {
        self.0.out_signals()
    }

    fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
        self.0.resources(config)
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

        let mut queue = VecDeque::new();
        for _ in 0..self.1 {
            queue.push_back(
                self.0
                    .stateful(io, res, config, sync_writer, async_writer)?,
            );
        }

        let queue = Arc::new(Mutex::new(queue));

        {
            let queue = queue.clone();
            let blueprint =
                serde_cbor::to_vec(&self.0).wrap_err("Failed to serialize action blueprint.")?;

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
                .0
                .stateful(io, res, config, sync_writer, async_writer)?,
            queue,
            link: tx,
        }))
    }
}

impl StatefulAction for StatefulRepeat {
    impl_stateful!();

    fn props(&self) -> Props {
        (self.inner.props().bits() | INFINITE).into()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        self.inner.start(sync_writer, async_writer, state)
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
