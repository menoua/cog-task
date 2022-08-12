use crate::action::{Action, StatefulAction, StatefulActionMsg, Style};
use crate::config::Config;
use crate::error;
use crate::error::Error::{InternalError, InvalidNameError};
use crate::io::IO;
use crate::logger::LoggerMsg;
use crate::resource::ResourceMap;
use crate::scheduler::{Monitor, SchedulerMsg};
use crate::server::ServerMsg;
use crate::style;
use crate::util::{f32_with_precision, f64_with_precision, str_with_precision};
use iced::alignment::{Horizontal, Vertical};
use iced::pure::widget::{
    Button, Checkbox, Column, Container, Radio, Row, Rule, Scrollable, Slider, Space, Text,
    TextInput,
};
use iced::pure::Element;
use iced::{Alignment, Length};
use iced_native::Command;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::ops::RangeInclusive;
use std::path::PathBuf;

const SHIFT: usize = 0x1000;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    #[serde(default = "defaults::group")]
    group: String,
    list: Vec<QItem>,
    #[serde(default)]
    style: String,
}

mod defaults {
    #[inline(always)]
    pub fn group() -> String {
        "questions".to_owned()
    }

    #[inline(always)]
    pub fn lines() -> u32 {
        3
    }

    #[inline(always)]
    pub fn columns() -> u32 {
        10
    }

    #[inline(always)]
    pub fn precision() -> u8 {
        3
    }
}

impl Action for Question {
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
        if self.group.is_empty() {
            Err(InvalidNameError(
                "Question `group` cannot be an empty string".to_owned(),
            ))
        } else {
            Ok(Box::new(StatefulQuestion {
                id,
                done: false,
                group: self.group.clone(),
                _style: Style::new("action-question", &self.style),
                list: self.list.iter().map(|q| q.stateful()).collect(),
            }))
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum QItem {
    SingleLine {
        id: String,
        prompt: String,
    },
    MultiLine {
        id: String,
        prompt: String,
        #[serde(default = "defaults::lines")]
        lines: u32,
    },
    SingleChoice {
        id: String,
        prompt: String,
        options: Vec<String>,
        #[serde(default = "defaults::columns")]
        columns: u32,
    },
    MultiChoice {
        id: String,
        prompt: String,
        options: Vec<String>,
        #[serde(default = "defaults::columns")]
        columns: u32,
    },
    Slider {
        id: String,
        prompt: String,
        range: (f32, f32),
        step: f32,
        #[serde(default = "defaults::precision")]
        precision: u8,
    },
}

impl QItem {
    fn stateful(&self) -> StatefulQItem {
        match self {
            QItem::SingleLine { id, prompt } => StatefulQItem::SingleLine {
                id: id.clone(),
                prompt: prompt.clone(),
                input: String::new(),
            },
            QItem::MultiLine { id, prompt, lines } => StatefulQItem::MultiLine {
                id: id.clone(),
                prompt: prompt.clone(),
                lines: *lines,
                input: String::new(),
            },
            QItem::SingleChoice {
                id,
                prompt,
                options,
                columns,
            } => StatefulQItem::SingleChoice {
                id: id.clone(),
                prompt: prompt.clone(),
                options: options.clone(),
                choice: None,
                columns: *columns,
            },
            QItem::MultiChoice {
                id,
                prompt,
                options,
                columns,
            } => StatefulQItem::MultiChoice {
                id: id.clone(),
                prompt: prompt.clone(),
                options: options.clone(),
                choice: vec![false; options.len()],
                columns: *columns,
            },
            QItem::Slider {
                id,
                prompt,
                range,
                step,
                precision,
            } => StatefulQItem::Slider {
                id: id.clone(),
                prompt: prompt.clone(),
                range: (
                    f32_with_precision(range.0, *precision),
                    f32_with_precision(range.1, *precision),
                ),
                step: *step,
                choice: range.0,
                precision: *precision,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum StatefulQItem {
    SingleLine {
        id: String,
        prompt: String,
        input: String,
    },
    MultiLine {
        id: String,
        prompt: String,
        lines: u32,
        input: String,
    },
    SingleChoice {
        id: String,
        prompt: String,
        options: Vec<String>,
        choice: Option<usize>,
        columns: u32,
    },
    MultiChoice {
        id: String,
        prompt: String,
        options: Vec<String>,
        choice: Vec<bool>,
        columns: u32,
    },
    Slider {
        id: String,
        prompt: String,
        range: (f32, f32),
        step: f32,
        choice: f32,
        precision: u8,
    },
}

impl StatefulQItem {
    fn set_answer_i32(&mut self, which: usize, _value: i32) -> Result<(), error::Error> {
        Err(InternalError(format!(
            "Error in question callback for item {which}"
        )))
    }

    fn set_answer_f32(&mut self, which: usize, value: f32) -> Result<(), error::Error> {
        match self {
            StatefulQItem::Slider { choice, .. } => {
                *choice = value;
            }
            _ => {
                return Err(InternalError(format!(
                    "Error in question callback for item {which}"
                )))
            }
        }
        Ok(())
    }

    fn set_answer_bool(&mut self, which: usize, value: bool) -> Result<(), error::Error> {
        match self {
            StatefulQItem::SingleChoice { choice, .. } => {
                if value {
                    *choice = Some(which);
                }
            }
            StatefulQItem::MultiChoice { choice, .. } => {
                choice[which] = value;
            }
            _ => {
                return Err(InternalError(format!(
                    "Error in question callback for item {which}"
                )))
            }
        }
        Ok(())
    }

    fn set_answer_str(&mut self, which: usize, value: String) -> Result<(), error::Error> {
        match self {
            StatefulQItem::SingleLine { input, .. } => {
                *input = value;
            }
            StatefulQItem::MultiLine { input, .. } => {
                *input = value;
            }
            _ => {
                return Err(InternalError(format!(
                    "Error in question callback for item {which}"
                )))
            }
        }
        Ok(())
    }

    fn to_string(&self) -> (String, Value) {
        let name = match self {
            StatefulQItem::SingleLine { id, .. }
            | StatefulQItem::MultiLine { id, .. }
            | StatefulQItem::SingleChoice { id, .. }
            | StatefulQItem::MultiChoice { id, .. }
            | StatefulQItem::Slider { id, .. } => id.to_owned(),
        };

        let value = match self {
            StatefulQItem::SingleLine { input, .. } | StatefulQItem::MultiLine { input, .. } => {
                Value::String(input.to_owned())
            }
            StatefulQItem::SingleChoice {
                choice, options, ..
            } => {
                if let Some(choice) = choice {
                    Value::String(options[*choice].to_owned())
                } else {
                    Value::Null
                }
            }
            StatefulQItem::MultiChoice {
                choice, options, ..
            } => Value::Array(
                choice
                    .iter()
                    .enumerate()
                    .filter_map(|(i, checked)| {
                        if *checked {
                            Some(Value::String(options[i].to_owned()))
                        } else {
                            None
                        }
                    })
                    .collect(),
            ),
            StatefulQItem::Slider {
                choice, precision, ..
            } => Value::Number({
                Number::from_f64(f64_with_precision(*choice, *precision)).unwrap()
            }),
        };

        (name, value)
    }
}

#[derive(Debug)]
pub struct StatefulQuestion {
    id: usize,
    done: bool,
    group: String,
    _style: Style,
    list: Vec<StatefulQItem>,
}

impl StatefulAction for StatefulQuestion {
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
        Ok(match msg {
            StatefulActionMsg::Update(0x00) => {
                self.done = true;
                let answers = LoggerMsg::Extend(
                    self.group.clone(),
                    self.list.iter().map(|q| q.to_string()).collect(),
                );
                Command::batch([
                    Command::perform(async move { answers }, LoggerMsg::wrap),
                    Command::perform(async { SchedulerMsg::Advance }, SchedulerMsg::wrap),
                ])
            }
            StatefulActionMsg::UpdateInt(code, value) => {
                self.list[code as usize / SHIFT].set_answer_i32(code as usize % SHIFT, value)?;
                Command::none()
            }
            StatefulActionMsg::UpdateFloat(code, value) => {
                self.list[code as usize / SHIFT].set_answer_f32(code as usize % SHIFT, value)?;
                Command::none()
            }
            StatefulActionMsg::UpdateBool(code, value) => {
                self.list[code as usize / SHIFT].set_answer_bool(code as usize % SHIFT, value)?;
                Command::none()
            }
            StatefulActionMsg::UpdateString(code, value) => {
                self.list[code as usize / SHIFT].set_answer_str(code as usize % SHIFT, value)?;
                Command::none()
            }
            _ => Command::none(),
        })
    }

    fn view(&self, _scale_factor: f32) -> Result<Element<'_, ServerMsg>, error::Error> {
        let mut content: Column<ServerMsg> = Column::new()
            .spacing(20)
            .padding([0, 10])
            .align_items(Alignment::Start);

        for (i, question) in self.list.iter().enumerate() {
            if i > 0 {
                content = content.push(Rule::horizontal(5));
            }

            let prompt = match question {
                StatefulQItem::SingleLine { prompt, .. }
                | StatefulQItem::MultiLine { prompt, .. }
                | StatefulQItem::SingleChoice { prompt, .. }
                | StatefulQItem::MultiChoice { prompt, .. }
                | StatefulQItem::Slider { prompt, .. } => Text::new(prompt).size(24),
            };

            let fields = match question {
                StatefulQItem::SingleLine { input, .. } => Container::new(
                    TextInput::new("Your answer goes here", input, move |s| {
                        StatefulActionMsg::UpdateString(SHIFT * i, s).wrap()
                    })
                    .size(24)
                    .padding([3, 6]),
                ),
                StatefulQItem::MultiLine { input, .. } => Container::new(
                    TextInput::new("Your answer goes here", input, move |s| {
                        StatefulActionMsg::UpdateString(SHIFT * i, s).wrap()
                    })
                    .size(24)
                    .padding([3, 6]),
                ),
                StatefulQItem::SingleChoice {
                    options,
                    choice,
                    columns,
                    ..
                } => {
                    let options: Vec<_> = options
                        .iter()
                        .enumerate()
                        .map(|(j, o)| {
                            Radio::new(j, o.to_string(), *choice, |e| {
                                StatefulActionMsg::UpdateBool(SHIFT * i + e, true).wrap()
                            })
                            .style(style::Radio)
                            .size(20)
                        })
                        .collect();
                    Container::new(style::grid(options, *columns as usize, 10, 30))
                }
                StatefulQItem::MultiChoice {
                    options,
                    columns,
                    choice,
                    ..
                } => {
                    let options: Vec<_> = options
                        .iter()
                        .enumerate()
                        .map(|(j, o)| {
                            Checkbox::new(choice[j], o.to_string(), move |e| {
                                StatefulActionMsg::UpdateBool(SHIFT * i + j, e).wrap()
                            })
                        })
                        .collect();
                    Container::new(style::grid(options, *columns as usize, 10, 30))
                }
                StatefulQItem::Slider {
                    range,
                    step,
                    choice,
                    precision,
                    ..
                } => Container::new(
                    Row::new()
                        .align_items(Alignment::Center)
                        .spacing(20)
                        .padding([0, 100])
                        .push(
                            Text::new(str_with_precision(range.0, *precision))
                                .size(24)
                                .vertical_alignment(Vertical::Center),
                        )
                        .push(
                            Slider::new(RangeInclusive::new(range.0, range.1), *choice, move |v| {
                                StatefulActionMsg::UpdateFloat(SHIFT * i, v).wrap()
                            })
                            .step(*step)
                            .width(Length::Units(250)),
                        )
                        .push(
                            Text::new(str_with_precision(range.1, *precision))
                                .size(24)
                                .vertical_alignment(Vertical::Center),
                        )
                        .push(Space::with_width(Length::Fill))
                        .push(
                            Text::new(str_with_precision(*choice, *precision))
                                .size(24)
                                .vertical_alignment(Vertical::Center),
                        ),
                ),
            }
            .width(Length::Fill)
            .padding([0, 10])
            .center_x();

            content = content.push(Column::new().spacing(20).push(prompt).push(fields));
        }

        let content = Scrollable::new(content);
        let submit = Button::new(
            Text::new("Submit")
                .size(24)
                .horizontal_alignment(Horizontal::Center)
                .vertical_alignment(Vertical::Center),
        )
        .padding([15, 40])
        .style(style::Submit)
        .on_press(StatefulActionMsg::Update(0x00).wrap());

        Ok(Container::new(
            Column::new()
                .max_width(800)
                .align_items(Alignment::Center)
                .spacing(25)
                .push(content.height(Length::FillPortion(9)))
                .push(Rule::horizontal(3))
                .push(
                    Container::new(submit)
                        .width(Length::Fill)
                        .height(Length::FillPortion(1))
                        .center_x()
                        .center_y(),
                ),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(50)
        .center_x()
        .center_y()
        .into())
    }
}
