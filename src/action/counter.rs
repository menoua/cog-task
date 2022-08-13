use crate::action::{Action, StatefulAction, StatefulActionMsg};
use crate::config::Config;
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::{Monitor, SchedulerMsg};
use crate::server::ServerMsg;
use crate::{error, style};
use iced::pure::widget::{Button, Container, Text};
use iced::pure::Element;
use iced::{Command, Length};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Counter {
    #[serde(default = "defaults::from")]
    from: u32,
    #[serde(default)]
    style: String,
}

mod defaults {
    #[inline(always)]
    pub fn from() -> u32 {
        3
    }
}

impl From<u32> for Counter {
    fn from(i: u32) -> Self {
        Self {
            from: i,
            style: "".to_owned(),
        }
    }
}

impl Action for Counter {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![]
    }

    fn stateful(
        &self,
        id: usize,
        _res: &ResourceMap,
        _config: &Config,
        _io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        Ok(Box::new(StatefulCounter {
            id,
            done: false,
            count: self.from,
            // style: Style::new("action-counter", &self.style),
        }))
    }
}

#[derive(Debug)]
pub struct StatefulCounter {
    id: usize,
    done: bool,
    count: u32,
}

impl StatefulAction for StatefulCounter {
    #[inline(always)]
    fn id(&self) -> usize {
        self.id
    }

    #[inline(always)]
    fn is_over(&self) -> bool {
        self.done || self.count == 0
    }

    #[inline(always)]
    fn is_visual(&self) -> bool {
        true
    }

    #[inline(always)]
    fn is_static(&self) -> bool {
        false
    }

    #[inline(always)]
    fn monitors(&self) -> Option<Monitor> {
        None
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        self.done = true;
        Ok(())
    }

    fn update(&mut self, msg: StatefulActionMsg) -> Result<Command<ServerMsg>, error::Error> {
        if let StatefulActionMsg::Update(0x00) = msg {
            self.count -= 1;
        }
        if self.count == 0 {
            self.done = true;
            Ok(Command::perform(async {}, |()| {
                SchedulerMsg::Advance.wrap()
            }))
        } else {
            Ok(Command::none())
        }
    }

    fn view(&self, _scale_factor: f32) -> Result<Element<'_, ServerMsg>, error::Error> {
        Ok(Container::new(
            Button::new(Text::new(format!("Click me {} more times", self.count)).size(34))
                .padding([15, 60])
                .on_press(StatefulActionMsg::Update(0x00).wrap())
                .style(style::Select),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into())
    }
}
