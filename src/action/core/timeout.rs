use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::comm::QWriter;
use crate::resource::{ResourceAddr, ResourceMap};
use crate::server::{AsyncSignal, Config, State, SyncSignal, IO};
use crate::util::spin_sleeper;
use eframe::egui::Ui;
use eyre::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize)]
pub struct Timeout(f32, Box<dyn Action>);

stateful!(Timeout {
    duration: Duration,
    timeout_over: Arc<Mutex<bool>>,
    inner: Box<dyn StatefulAction>,
});

impl Action for Timeout {
    #[inline(always)]
    fn resources(&self, config: &Config) -> Vec<ResourceAddr> {
        self.1.resources(config)
    }

    fn stateful(
        &self,
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        let inner = self
            .1
            .stateful(io, res, config, sync_writer, async_writer)?;

        Ok(Box::new(StatefulTimeout {
            done: inner.is_over()?,
            duration: Duration::from_secs_f32(self.0),
            timeout_over: Arc::new(Mutex::new(false)),
            inner,
        }))
    }
}

impl StatefulAction for StatefulTimeout {
    impl_stateful!();

    fn props(&self) -> Props {
        let bits = self.inner.props().bits();
        (bits & !INFINITE).into()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<(), Error> {
        if self.done {
            sync_writer.push(SyncSignal::UpdateGraph);
        } else {
            self.inner.start(sync_writer, async_writer, state)?;

            let dur = self.duration;
            let timeout_over = self.timeout_over.clone();
            let mut sync_writer = sync_writer.clone();
            thread::spawn(move || {
                spin_sleeper().sleep(dur);
                *timeout_over.lock().unwrap() = true;
                sync_writer.push(SyncSignal::UpdateGraph);
            });
        }

        Ok(())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<(), Error> {
        self.inner
            .update(signal, sync_writer, async_writer, state)?;
        if self.inner.is_over()?
            || (matches!(signal, ActionSignal::UpdateGraph) && *self.timeout_over.lock().unwrap())
        {
            self.done = true;
        }
        Ok(())
    }

    fn show(
        &mut self,
        ui: &mut Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<(), Error> {
        self.inner.show(ui, sync_writer, async_writer, state)
    }

    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<(), Error> {
        self.inner.stop(sync_writer, async_writer, state)?;
        self.done = true;
        Ok(())
    }
}
