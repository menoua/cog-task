use crate::config::{Config, LogCondition};
use crate::error;
use crate::error::Error::{ActionViewError, InvalidNameError};
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::monitor::{Event, Monitor};
use crate::scheduler::SchedulerMsg;
use crate::server::ServerMsg;
use iced::pure::Element;
use iced::Command;
use std::fmt::Debug;
use std::path::{Path, PathBuf};

pub mod audio;
pub mod counter;
pub mod de;
pub mod image;
pub mod instruction;
pub mod key_logger;
pub mod nop;
pub mod question;
pub mod simple;
pub mod stream;
pub mod video;

pub use audio::Audio;
pub use counter::Counter;
pub use image::{Fixation, Image};
pub use instruction::Instruction;
pub use key_logger::KeyLogger;
pub use nop::Nop;
pub use question::Question;
pub use simple::Simple;
pub use stream::Stream;
pub use video::Video;

pub trait Action: Debug {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![]
    }

    #[inline(always)]
    fn init(&self) -> Result<(), error::Error> {
        Ok(())
    }

    fn stateful(
        &self,
        id: usize,
        res: &ResourceMap,
        config: &Config,
        io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error>;

    #[inline(always)]
    fn evolve(
        &self,
        _root_dir: &Path,
        _config: &Config,
    ) -> Result<Option<Box<dyn Action>>, error::Error> {
        Ok(None)
    }
}

pub trait StatefulAction: Debug {
    fn id(&self) -> usize;

    #[inline(always)]
    fn is_over(&self) -> Result<bool, error::Error> {
        Ok(false)
    }

    #[inline(always)]
    fn is_visual(&self) -> bool {
        true
    }

    #[inline(always)]
    fn is_static(&self) -> bool {
        true
    }

    #[inline(always)]
    fn monitors(&self) -> Option<Monitor> {
        None
    }

    fn stop(&mut self) -> Result<(), error::Error>;

    #[inline(always)]
    fn start(&mut self) -> Result<Command<ServerMsg>, error::Error> {
        Ok(Command::none())
    }

    #[inline(always)]
    fn update(&mut self, _msg: StatefulActionMsg) -> Result<Command<ServerMsg>, error::Error> {
        Ok(Command::none())
    }

    #[inline(always)]
    fn view(&self, _scale_factor: f32) -> Result<Element<'_, ServerMsg>, error::Error> {
        Err(ActionViewError(format!(
            "View not implemented for action `{}`",
            self.id()
        )))
    }
}

#[derive(Debug, Clone)]
pub enum StatefulActionMsg {
    Update(usize),
    UpdateInt(usize, i32),
    UpdateUInt(usize, u32),
    UpdateFloat(usize, f32),
    UpdateBool(usize, bool),
    UpdateString(usize, String),
    UpdateEvent(Event),
}

impl StatefulActionMsg {
    #[inline(always)]
    pub fn wrap(self) -> ServerMsg {
        ServerMsg::Relay(SchedulerMsg::Relay(self))
    }
}

#[derive(Debug)]
pub struct ExtAction {
    id: Option<String>,
    action: Box<dyn Action>,
    log_when: Option<LogCondition>,
}

impl ExtAction {
    pub fn new(
        id: Option<String>,
        action: Box<dyn Action>,
        log_when: Option<LogCondition>,
    ) -> Self {
        Self {
            id,
            action,
            log_when,
        }
    }

    #[inline(always)]
    pub fn id(&self) -> Option<&String> {
        self.id.as_ref()
    }

    #[inline(always)]
    pub fn inner(&self) -> &dyn Action {
        self.action.as_ref()
    }

    #[inline(always)]
    pub fn log_when(&self) -> &Option<LogCondition> {
        &self.log_when
    }

    pub fn init(&mut self, root_dir: &Path, config: &Config) -> Result<(), error::Error> {
        if let Some(id) = &self.id {
            ExtAction::verify_id(id)?;
        }
        if let Some(descendent) = self.action.evolve(root_dir, config)? {
            self.action = descendent;
        }
        self.action.init()?;
        Ok(())
    }

    fn verify_id(id: &str) -> Result<(), error::Error> {
        if id.is_empty() {
            Err(InvalidNameError(
                "Action `id` cannot be the empty string. You can use `None` to ignore the name."
                    .to_owned(),
            ))
        } else if id.split_whitespace().count() > 1 {
            Err(InvalidNameError(format!(
                "Action `id` cannot contain whitespaces: '{id}'"
            )))
        } else if !id.chars().next().unwrap().is_alphabetic() {
            Err(InvalidNameError(format!(
                "Action `id` cannot start with a non-alphabetic character: '{id}'"
            )))
        } else if !id
            .chars()
            .all(|c| c.is_alphabetic() || c.is_alphanumeric() | "-_".contains(c))
        {
            Err(InvalidNameError(format!(
                "Action `id` characters need to be alphanumeric or one of '-' or '_': '{id}'"
            )))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
struct Style(Vec<String>);
impl Style {
    fn new(base: &str, custom: &str) -> Self {
        Self(
            format!("{base} {custom}")
                .split_whitespace()
                .map(|c| c.to_owned())
                .collect(),
        )
    }

    // fn to_vec(&self) -> Vec<String> {
    //     self.0.clone()
    // }
}
