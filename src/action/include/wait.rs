use crate::action::{
    Action, ActionEnum, ActionSignal, Props, StatefulAction, StatefulActionEnum, DEFAULT, INFINITE,
};
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
use std::sync::mpsc::{self, Sender};
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
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<StatefulActionEnum, error::Error> {
        Ok(StatefulWait {
            id: 0,
            done: Arc::new(Mutex::new(Ok(false))),
            duration: Duration::from_secs_f32(self.0),
        }
        .into())
    }
}

impl StatefulAction for StatefulWait {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        DEFAULT.into()
    }

    #[inline(always)]
    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        let mut done = self.done.clone();
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
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn show(
        &mut self,
        ui: &mut Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        Ok(())
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        *self.done.lock().unwrap() = Ok(true);
        Ok(())
    }
}
