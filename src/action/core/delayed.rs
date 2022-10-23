use crate::action::{Action, StatefulAction, DEFAULT};
use crate::action::{ActionSignal, Props};
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::io::IO;
use crate::queue::QWriter;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::scheduler::State;
use crate::util::spin_sleeper;
use eframe::egui::Ui;
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
    has_begun: bool,
    inner: Box<dyn StatefulAction>,
});

impl Action for Delayed {
    #[inline(always)]
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
            has_begun: false,
            inner,
        }))
    }
}

impl StatefulAction for StatefulDelayed {
    impl_stateful!();

    fn props(&self) -> Props {
        if self.has_begun {
            self.inner.props()
        } else {
            DEFAULT.into()
        }
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        if self.done {
            sync_writer.push(SyncSignal::UpdateGraph);
        } else {
            let wait_over = self.wait_over.clone();
            let dur = self.duration;
            let mut sync_writer = sync_writer.clone();
            thread::spawn(move || {
                spin_sleeper().sleep(dur);
                *wait_over.lock().unwrap() = true;
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
        if *self.wait_over.lock().unwrap() {
            if !self.has_begun {
                self.inner.start(sync_writer, async_writer, state)?;
                self.has_begun = true;
            } else {
                self.inner
                    .update(signal, sync_writer, async_writer, state)?;
            }

            if self.inner.is_over()? {
                self.done = true;
            }
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
        if self.has_begun {
            self.inner.show(ui, sync_writer, async_writer, state)
        } else {
            Ok(())
        }
    }

    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<(), Error> {
        if *self.wait_over.lock().unwrap() {
            self.inner.stop(sync_writer, async_writer, state)?;
        }
        self.done = true;
        Ok(())
    }
}
