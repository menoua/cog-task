use crate::action::{Action, StatefulAction};
use crate::action::{ActionSignal, Props};
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use crate::util::spin_sleeper;
use eframe::egui::{CursorIcon, Ui};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize)]
pub struct Delayed(f32, Box<dyn Action>);

stateful!(Delayed {
    duration: Duration,
    wait_over: Arc<Mutex<bool>>,
    inner: Box<dyn StatefulAction>,
});

impl Action for Delayed {
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

        Ok(Box::new(StatefulDelayed {
            done: inner.is_over()?,
            duration: Duration::from_secs_f32(self.0),
            wait_over: Arc::new(Mutex::new(false)),
            inner,
        }))
    }
}

impl StatefulAction for StatefulDelayed {
    impl_stateful!();

    fn props(&self) -> Props {
        self.inner.props()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        let wait_over = self.wait_over.clone();
        let dur = self.duration;
        let mut sync_writer = sync_writer.clone();
        thread::spawn(move || {
            spin_sleeper().sleep(dur);
            *wait_over.lock().unwrap() = true;
            sync_writer.push(SyncSignal::UpdateGraph)
        });
        Ok(())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        if *self.wait_over.lock().unwrap() {
            self.inner.update(signal, sync_writer, async_writer)?;
            match self.inner.is_over() {
                Ok(true) | Err(_) => self.done = true,
                Ok(false) => {}
            }
            Ok(())
        } else {
            self.inner.start(sync_writer, async_writer)
        }
    }

    fn show(
        &mut self,
        ui: &mut Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        if *self.wait_over.lock().unwrap() {
            self.inner.show(ui, sync_writer, async_writer)
        } else {
            ui.output().cursor_icon = CursorIcon::None;
            Ok(())
        }
    }

    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        if *self.wait_over.lock().unwrap() {
            self.inner.stop(sync_writer, async_writer)?;
        }
        self.done = true;
        Ok(())
    }
}
