use crate::action::par::Par;
use crate::action::wait::Wait;
use crate::action::{Action, ActionEnum, StatefulAction, StatefulActionEnum};
use crate::action::{ActionSignal, Image, Props, INFINITE};
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::error::Error::InternalError;
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use crate::util::spin_sleeper;
use eframe::egui::{CursorIcon, Ui};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize)]
pub struct Delayed(f32, Box<ActionEnum>);

stateful!(Delayed {
    duration: Duration,
    wait_over: Arc<Mutex<bool>>,
    inner: Box<StatefulActionEnum>,
});

impl Action for Delayed {
    #[inline]
    fn resources(&self, config: &Config) -> Vec<PathBuf> {
        self.1.inner().resources(config)
    }

    fn init(&mut self, root_dir: &Path, config: &Config) -> Result<(), error::Error> {
        self.1.inner_mut().init(root_dir, config)
    }

    fn stateful(
        &self,
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<StatefulActionEnum, error::Error> {
        let inner = self
            .1
            .inner()
            .stateful(io, res, config, sync_writer, async_writer)?;

        Ok(StatefulDelayed {
            id: 0,
            done: inner.inner().is_over()?,
            duration: Duration::from_secs_f32(self.0),
            wait_over: Arc::new(Mutex::new(false)),
            inner: Box::new(inner),
        }
        .into())
    }
}

impl StatefulAction for StatefulDelayed {
    impl_stateful!();

    fn props(&self) -> Props {
        self.inner.inner().props()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        let mut wait_over = self.wait_over.clone();
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
            self.inner
                .inner_mut()
                .update(signal, sync_writer, async_writer)?;
            match self.inner.inner().is_over() {
                Ok(true) | Err(_) => self.done = true,
                Ok(false) => {}
            }
            Ok(())
        } else {
            self.inner.inner_mut().start(sync_writer, async_writer)
        }
    }

    fn show(
        &mut self,
        ui: &mut Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        if *self.wait_over.lock().unwrap() {
            self.inner.inner_mut().show(ui, sync_writer, async_writer)
        } else {
            ui.output().cursor_icon = CursorIcon::None;
            Ok(())
        }
    }

    fn stop(&mut self) -> Result<(), Error> {
        self.inner.inner_mut().stop()?;
        self.done = true;
        Ok(())
    }
}
