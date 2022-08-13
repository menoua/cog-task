use crate::action::{Action, StatefulAction, StatefulActionMsg};
use crate::config::Config;
use crate::error::Error::TaskDefinitionError;
use crate::io::IO;
use crate::resource::text::Justification;
use crate::resource::{text::text_or_file, ResourceMap};
use crate::scheduler::{Monitor, SchedulerMsg};
use crate::server::ServerMsg;
use crate::{error, style};
use iced::pure::widget::{Button, Column, Container, Text};
use iced::pure::Element;
use iced::Length;
use iced_native::alignment::Vertical;
use iced_native::{Alignment, Command};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Instruction {
    text: String,
    #[serde(default = "defaults::justification")]
    justify: String,
    #[serde(default = "defaults::persistent")]
    #[serde(rename = "static")]
    persistent: bool,
    #[serde(default)]
    style: String,
}

mod defaults {
    #[inline(always)]
    pub fn justification() -> String {
        "Left".to_owned()
    }

    #[inline(always)]
    pub fn persistent() -> bool {
        false
    }
}

impl From<&str> for Instruction {
    fn from(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            justify: defaults::justification(),
            persistent: defaults::persistent(),
            style: "".to_owned(),
        }
    }
}

impl Action for Instruction {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        if let Some(path) = text_or_file(&self.text) {
            vec![path]
        } else {
            vec![]
        }
    }

    #[inline(always)]
    fn init(&self) -> Result<(), error::Error> {
        match self.justify.to_lowercase().as_str() {
            "left" | "center" | "right" => Ok(()),
            j => Err(TaskDefinitionError(format!(
                "Unknown justification value '{j}' (should be 'left', 'center', or 'right')"
            )))?,
        }
    }

    fn stateful(
        &self,
        id: usize,
        res: &ResourceMap,
        _config: &Config,
        _io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        let text = res.fetch_text(&self.text)?;
        let justify = match self.justify.to_lowercase().as_str() {
            "left" => Justification::Left,
            "center" => Justification::Center,
            "right" => Justification::Right,
            j => Err(TaskDefinitionError(format!(
                "Unknown justification value '{j}' (should be 'left', 'center', or 'right')"
            )))?,
        };

        Ok(Box::new(StatefulInstruction {
            id,
            done: false,
            text,
            justify,
            persistent: self.persistent,
            // style: Style::new("action-instruction", &self.style),
        }))
    }
}

#[derive(Debug)]
pub struct StatefulInstruction {
    id: usize,
    done: bool,
    text: String,
    justify: Justification,
    persistent: bool,
}

impl StatefulAction for StatefulInstruction {
    #[inline(always)]
    fn id(&self) -> usize {
        self.id
    }

    #[inline(always)]
    fn is_over(&self) -> bool {
        self.done
    }

    #[inline(always)]
    fn is_visual(&self) -> bool {
        true
    }

    #[inline(always)]
    fn is_static(&self) -> bool {
        self.persistent
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
            self.done = true;
            Ok(Command::perform(async {}, |()| {
                SchedulerMsg::Advance.wrap()
            }))
        } else {
            Ok(Command::none())
        }
    }

    fn view(&self, _scale_factor: f32) -> Result<Element<'_, ServerMsg>, error::Error> {
        let content = Column::new()
            .spacing(75)
            .align_items(Alignment::Center)
            .push(
                Text::new(&self.text)
                    .size(34)
                    .horizontal_alignment(self.justify.into())
                    .vertical_alignment(Vertical::Center),
            )
            .max_width(1200);

        Ok(Container::new(if self.persistent {
            content
        } else {
            content.push(
                Button::new(Text::new("Next").size(34))
                    .padding([15, 60])
                    .on_press(StatefulActionMsg::Update(0x00).wrap())
                    .style(style::Submit),
            )
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into())
    }
}
