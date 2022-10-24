use crate::action::{Action, ActionSignal, Props, StatefulAction, DEFAULT};
use crate::config::Config;
use crate::io::IO;
use crate::queue::QWriter;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::scheduler::State;
use crate::util::spin_sleeper;
use eframe::egui::Ui;
use eyre::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Wait(f32);

stateful_arc!(Wait { duration: Duration });

impl Wait {
    pub fn new(dur: f32) -> Self {
        Self(dur)
    }
}

impl Action for Wait {
    fn stateful(
        &self,
        _io: &IO,
        _res: &ResourceMap,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Ok(Box::new(StatefulWait {
            done: Arc::new(Mutex::new(Ok(false))),
            duration: Duration::from_secs_f32(self.0),
        }))
    }
}

impl StatefulAction for StatefulWait {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        DEFAULT.into()
    }

    #[inline]
    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<()> {
        let done = self.done.clone();
        let duration = self.duration;
        let mut sync_writer = sync_writer.clone();
        thread::spawn(move || {
            spin_sleeper().sleep(duration);
            *done.lock().unwrap() = Ok(true);
            sync_writer.push(SyncSignal::UpdateGraph);
        });
        Ok(())
    }

    fn update(
        &mut self,
        _signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn show(
        &mut self,
        _ui: &mut Ui,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        Ok(())
    }

    #[inline(always)]
    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<()> {
        *self.done.lock().unwrap() = Ok(true);
        Ok(())
    }
}
