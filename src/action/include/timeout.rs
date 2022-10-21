use crate::action::{Action, StatefulAction};
use crate::action::{ActionSignal, Props, INFINITE};
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use crate::util::spin_sleeper;
use eframe::egui::Ui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
    #[inline]
    fn resources(&self, config: &Config) -> Vec<PathBuf> {
        self.1.resources(config)
    }

    fn stateful(
        &self,
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
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
    ) -> Result<(), Error> {
        {
            let dur = self.duration;
            let timeout_over = self.timeout_over.clone();
            let mut sync_writer = sync_writer.clone();
            thread::spawn(move || {
                spin_sleeper().sleep(dur);
                *timeout_over.lock().unwrap() = true;
                sync_writer.push(SyncSignal::UpdateGraph);
            });
        }

        self.inner.start(sync_writer, async_writer)
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        self.inner.update(signal, sync_writer, async_writer)?;
        if self.inner.is_over()? && !self.done {
            self.done = true;
            sync_writer.push(SyncSignal::UpdateGraph);
        }

        if matches!(signal, ActionSignal::UpdateGraph) && !self.done {
            if *self.timeout_over.lock().unwrap() {
                self.inner.stop(sync_writer, async_writer)?;
                self.done = true;
                sync_writer.push(SyncSignal::UpdateGraph);
            }
        }
        Ok(())
    }

    fn show(
        &mut self,
        ui: &mut Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        self.inner.show(ui, sync_writer, async_writer)
    }

    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        self.inner.stop(sync_writer, async_writer)?;
        self.done = true;
        Ok(())
    }
}
