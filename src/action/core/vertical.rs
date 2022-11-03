use crate::action::core::stack::{Direction, Stack};
use crate::action::{Action, StatefulAction};
use crate::comm::QWriter;
use crate::resource::{IoManager, ResourceManager};
use crate::server::{AsyncSignal, Config, SyncSignal};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Vertical(Vec<Box<dyn Action>>, #[serde(default)] Vec<f32>);

impl Action for Vertical {
    fn init(self) -> Result<Box<dyn Action>>
    where
        Self: 'static + Sized,
    {
        Stack::new(self.0, Direction::Vertical, self.1).init()
    }

    fn stateful(
        &self,
        _io: &IoManager,
        _res: &ResourceManager,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Err(eyre!("Vertical can not be stateful."))
    }
}
