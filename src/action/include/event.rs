use crate::action::{
    Action, ActionEnum, ActionSignal, Props, StatefulAction, StatefulActionEnum, DEFAULT, INFINITE,
};
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::error::Error::{ActionEvolveError, InternalError, TaskDefinitionError};
use crate::io::IO;
use crate::logger::LoggerSignal;
use crate::resource::ResourceMap;
use crate::resource::ResourceValue::Text;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use eframe::egui::Ui;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
pub struct Event(String);

stateful!(Event { name: String });

impl Action for Event {
    #[inline(always)]
    fn stateful(
        &self,
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<StatefulActionEnum, error::Error> {
        Ok(StatefulEvent {
            id: 0,
            done: false,
            name: self.0.clone(),
        }
        .into())
    }
}

impl StatefulAction for StatefulEvent {
    impl_stateful!();

    fn props(&self) -> Props {
        INFINITE.into()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        async_writer.push(LoggerSignal::Append(
            "Event".to_owned(),
            (self.name.clone(), Value::String("start".to_owned())),
        ));
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

    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        async_writer.push(LoggerSignal::Append(
            "Event".to_owned(),
            (self.name.clone(), Value::String("stop".to_owned())),
        ));
        Ok(())
    }
}
