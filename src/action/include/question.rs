use crate::action::{Action, StatefulAction, StatefulActionMsg};
use crate::signal::QWriter;
use crate::config::Config;
use crate::error;
use crate::error::Error::{InternalError, InvalidNameError};
use crate::io::IO;
use crate::logger::LoggerCallback;
use crate::resource::ResourceMap;
use crate::scheduler::{AsyncCallback, SyncCallback};
use crate::style;
use crate::style::text::{body, button1, inactive};
use crate::util::{f32_with_precision, f64_with_precision, str_with_precision};
use eframe::egui;
use eframe::egui::{CentralPanel, Checkbox, Color32, RadioButton, ScrollArea, Slider, Stroke, TextEdit, Vec2, Widget};
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::ops::RangeInclusive;
use std::path::PathBuf;
use egui_extras::{Size, StripBuilder};
use crate::style::{CUSTOM_BLUE, Style, style_ui, TEXT_SIZE_BODY};
use crate::template::{center_x, header_body_controls};

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

stateful!(Question {
    group: String,
    list: Vec<StatefulQItem>,
});

mod defaults {
    #[inline(always)]
    pub fn group() -> String {
        "questions".to_owned()
    }

    #[inline(always)]
    pub fn lines() -> usize {
        3
    }

    #[inline(always)]
    pub fn columns() -> usize {
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
                // _style: Style::new("action-question", &self.style),
                list: self.list.iter().map(|q| q.stateful()).collect(),
            }))
        }
    }
}

impl StatefulAction for StatefulQuestion {
    impl_stateful!();

    #[inline(always)]
    fn is_visual(&self) -> bool {
        true
    }

    #[inline(always)]
    fn is_static(&self) -> bool {
        false
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        self.done = true;
        Ok(())
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_qw: &mut QWriter<SyncCallback>,
        async_qw: &mut QWriter<AsyncCallback>,
    ) -> Result<(), error::Error> {
        header_body_controls(
            ui,
            |mut strip| {
                strip.empty();
                strip.empty();
                strip.strip(|builder| {
                    center_x(
                        builder,
                        1520.0,
                        |ui| {
                            ScrollArea::vertical().show(ui, |ui| self.show_items(ui));
                        },
                    );
                });
                strip.empty();
                strip.strip(|builder| self.show_controls(builder, sync_qw, async_qw));
            }
        );

        Ok(())
    }
}

impl StatefulQuestion {
    fn show_items(&mut self, ui: &mut egui::Ui) {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(25.0);

            for (i, question) in self.list.iter_mut().enumerate() {
                if i > 0 {
                    ui.separator();
                }

                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(15.0);

                    match question {
                        StatefulQItem::SingleLine { prompt, .. }
                        | StatefulQItem::MultiLine { prompt, .. }
                        | StatefulQItem::SingleChoice { prompt, .. }
                        | StatefulQItem::MultiChoice { prompt, .. }
                        | StatefulQItem::Slider { prompt, .. } => ui.label(body(prompt.as_str())),
                    };

                    question.ui(ui);
                });
            }
        });
    }

    fn show_controls(&mut self, builder: StripBuilder, sync_qw: &mut QWriter<SyncCallback>, async_qw: &mut QWriter<AsyncCallback>) {
        enum Interaction {
            None,
            Submit,
        }

        let mut interaction = Interaction::None;

        center_x(builder, 250.0, |ui| {
            ui.horizontal_centered(|ui| {
                style_ui(ui, Style::SubmitButton);
                if ui.button(button1("Submit")).clicked() {
                    interaction = Interaction::Submit;
                }
            });
        });

        match interaction {
            Interaction::None => {}
            Interaction::Submit => {
                self.done = true;
                sync_qw.push(SyncCallback::UpdateGraph);
                async_qw.push(LoggerCallback::Extend(
                    self.group.clone(),
                    self.list.iter().map(|q| q.to_string()).collect(),
                ));
            }
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
        lines: usize,
    },
    SingleChoice {
        id: String,
        prompt: String,
        options: Vec<String>,
        #[serde(default = "defaults::columns")]
        columns: usize,
    },
    MultiChoice {
        id: String,
        prompt: String,
        options: Vec<String>,
        #[serde(default = "defaults::columns")]
        columns: usize,
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
        lines: usize,
        input: String,
    },
    SingleChoice {
        id: String,
        prompt: String,
        options: Vec<String>,
        choice: Option<usize>,
        columns: usize,
    },
    MultiChoice {
        id: String,
        prompt: String,
        options: Vec<String>,
        choice: Vec<bool>,
        columns: usize,
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
    fn ui(&mut self, ui: &mut egui::Ui) {
        match self {
            StatefulQItem::SingleLine { input, .. } => {
                ui.vertical_centered_justified(|ui| {
                    TextEdit::singleline(input).hint_text(inactive("Your answer goes here")).ui(ui);
                });
            }
            StatefulQItem::MultiLine { input, lines, .. } => {
                ui.vertical_centered_justified(|ui| {
                    TextEdit::multiline(input).hint_text(inactive("Your answer goes here")).desired_rows(*lines).ui(ui);
                });
            }
            StatefulQItem::SingleChoice { options, choice, columns, .. } => {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(45.0, 15.0);
                    ui.spacing_mut().icon_width = TEXT_SIZE_BODY  * 0.75;
                    ui.spacing_mut().icon_width_inner = TEXT_SIZE_BODY * 0.5;
                    ui.spacing_mut().icon_spacing = TEXT_SIZE_BODY * 0.25;
                    ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(2.5, Color32::DARK_GRAY);
                    ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(2.5, Color32::DARK_GRAY);
                    ui.visuals_mut().widgets.active.fg_stroke = Stroke::new(2.5, Color32::DARK_GRAY);
                    ui.visuals_mut().widgets.noninteractive.fg_stroke = Stroke::new(2.5, Color32::GRAY);

                    if *columns > 0 {
                        let mut i = 0;
                        ui.vertical_centered_justified(|ui| {
                            while i < options.len() {
                                ui.columns(*columns, |ui| {
                                    while i < options.len() {
                                        if RadioButton::new(*choice == Some(i), body(options[i].as_str()))
                                            .ui(&mut ui[i % *columns])
                                            .clicked() {
                                            *choice = Some(i);
                                        }

                                        i += 1;
                                    }
                                });
                            }
                        });
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            options.iter_mut().enumerate().for_each(|(i, option)| {
                                if RadioButton::new(*choice == Some(i), body(option.as_str()))
                                    .ui(ui)
                                    .clicked() {
                                    *choice = Some(i);
                                }
                            });
                        });
                    }
                });
            }
            StatefulQItem::MultiChoice { options, choice, columns, .. } => {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(45.0, 15.0);
                    ui.spacing_mut().icon_width = TEXT_SIZE_BODY  * 0.75;
                    ui.spacing_mut().icon_width_inner = TEXT_SIZE_BODY * 0.5;
                    ui.spacing_mut().icon_spacing = TEXT_SIZE_BODY * 0.25;
                    ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(2.5, Color32::DARK_GRAY);
                    ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(2.5, Color32::DARK_GRAY);
                    ui.visuals_mut().widgets.active.fg_stroke = Stroke::new(2.5, Color32::DARK_GRAY);
                    ui.visuals_mut().widgets.noninteractive.fg_stroke = Stroke::new(2.5, Color32::GRAY);

                    if *columns > 0 {
                        let mut i = 0;
                        ui.vertical_centered_justified(|ui| {
                            while i < options.len() {
                                ui.columns(*columns, |ui| {
                                    while i < options.len() {
                                        Checkbox::new(&mut choice[i], body(options[i].as_str()))
                                            .ui(&mut ui[i % *columns]);

                                        i += 1;
                                    }
                                });
                            }
                        });
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            options.iter_mut().enumerate().for_each(|(i, option)| {
                                Checkbox::new(&mut choice[i], body(option.as_str()))
                                    .ui(ui);
                            });
                        });
                    }
                });
            }
            StatefulQItem::Slider { range, step, choice, precision, .. } => {
                let range = RangeInclusive::new(
                    f32_with_precision(range.0, *precision),
                    f32_with_precision(range.1, *precision),
                );

                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().slider_width = 400.0;

                    ui.add_space(560.0);
                    Slider::new(choice, range)
                        .max_decimals(*precision as usize)
                        .step_by(*step as f64)
                        .clamp_to_range(true)
                        .ui(ui);
                });
            }
        }
    }
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
