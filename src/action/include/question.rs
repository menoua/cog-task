use crate::action::{Action, Props, StatefulAction, VISUAL, ActionEnum, StatefulActionEnum, ActionSignal};
use crate::signal::QWriter;
use crate::config::Config;
use crate::error;
use crate::error::Error::{InternalError, InvalidNameError};
use crate::io::IO;
use crate::logger::LoggerSignal;
use crate::resource::ResourceMap;
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
use crate::error::Error;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
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
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<StatefulActionEnum, error::Error> {
        if self.group.is_empty() {
            Err(InvalidNameError(
                "Question `group` cannot be an empty string".to_owned(),
            ))
        } else {
            Ok(StatefulQuestion {
                id: 0,
                done: false,
                group: self.group.clone(),
                // _style: Style::new("action-question", &self.style),
                list: self.list.iter().map(|q| q.stateful()).collect(),
            }.into())
        }
    }
}

impl StatefulAction for StatefulQuestion {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        VISUAL.into()
    }

    fn start(&mut self, sync_writer: &mut QWriter<SyncSignal>, async_writer: &mut QWriter<AsyncSignal>) -> Result<(), Error> {
        Ok(())
    }

    fn update(&mut self, signal: &ActionSignal, sync_writer: &mut QWriter<SyncSignal>, async_writer: &mut QWriter<AsyncSignal>) -> Result<(), Error> {
        Ok(())
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
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
                strip.strip(|builder| self.show_controls(builder, sync_writer, async_writer));
            }
        );

        Ok(())
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        self.done = true;
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

    fn show_controls(&mut self, builder: StripBuilder, sync_writer: &mut QWriter<SyncSignal>, async_writer: &mut QWriter<AsyncSignal>) {
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
                sync_writer.push(SyncSignal::UpdateGraph);
                async_writer.push(LoggerSignal::Extend(
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
                Self::show_single_line(ui, input)
            }
            StatefulQItem::MultiLine { input, lines, .. } => {
                Self::show_multi_line(ui, input, *lines)
            }
            StatefulQItem::SingleChoice { options, choice, columns, .. } => {
                Self::show_single_choice(ui, &options, choice, *columns)
            }
            StatefulQItem::MultiChoice { options, choice, columns, .. } => {
                Self::show_multi_choice(ui, &options, choice, *columns)
            }
            StatefulQItem::Slider { range, step, choice, precision, .. } => {
                Self::show_slider(ui, *range, *step, choice, *precision)
            }
        }
    }

    fn show_single_line(ui: &mut egui::Ui, input: &mut String) {
        ui.vertical_centered_justified(|ui| {
            TextEdit::singleline(input).hint_text(inactive("Your answer goes here")).ui(ui);
        });
    }

    fn show_multi_line(ui: &mut egui::Ui, input: &mut String, lines: usize) {
        ui.vertical_centered_justified(|ui| {
            TextEdit::multiline(input).hint_text(inactive("Your answer goes here")).desired_rows(lines).ui(ui);
        });
    }

    fn show_single_choice(ui: &mut egui::Ui, options: &[String], choice: &mut Option<usize>, columns: usize) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(45.0, 15.0);
            ui.spacing_mut().icon_width = TEXT_SIZE_BODY  * 0.75;
            ui.spacing_mut().icon_width_inner = TEXT_SIZE_BODY * 0.5;
            ui.spacing_mut().icon_spacing = TEXT_SIZE_BODY * 0.25;
            ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(2.5, Color32::DARK_GRAY);
            ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(2.5, Color32::DARK_GRAY);
            ui.visuals_mut().widgets.active.fg_stroke = Stroke::new(2.5, Color32::DARK_GRAY);
            ui.visuals_mut().widgets.noninteractive.fg_stroke = Stroke::new(2.5, Color32::GRAY);

            if columns > 0 {
                let mut i = 0;
                ui.vertical_centered_justified(|ui| {
                    while i < options.len() {
                        ui.columns(columns, |ui| {
                            while i < options.len() {
                                if RadioButton::new(*choice == Some(i), body(options[i].as_str()))
                                    .ui(&mut ui[i % columns])
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
                    options.iter().enumerate().for_each(|(i, option)| {
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

    fn show_multi_choice(ui: &mut egui::Ui, options: &[String], choice: &mut Vec<bool>, columns: usize) {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(45.0, 15.0);
            ui.spacing_mut().icon_width = TEXT_SIZE_BODY  * 0.75;
            ui.spacing_mut().icon_width_inner = TEXT_SIZE_BODY * 0.5;
            ui.spacing_mut().icon_spacing = TEXT_SIZE_BODY * 0.25;
            ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(2.5, Color32::DARK_GRAY);
            ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(2.5, Color32::DARK_GRAY);
            ui.visuals_mut().widgets.active.fg_stroke = Stroke::new(2.5, Color32::DARK_GRAY);
            ui.visuals_mut().widgets.noninteractive.fg_stroke = Stroke::new(2.5, Color32::GRAY);

            if columns > 0 {
                let mut i = 0;
                ui.vertical_centered_justified(|ui| {
                    while i < options.len() {
                        ui.columns(columns, |ui| {
                            while i < options.len() {
                                Checkbox::new(&mut choice[i], body(options[i].as_str()))
                                    .ui(&mut ui[i % columns]);

                                i += 1;
                            }
                        });
                    }
                });
            } else {
                ui.horizontal_wrapped(|ui| {
                    options.iter().enumerate().for_each(|(i, option)| {
                        Checkbox::new(&mut choice[i], body(option.as_str()))
                            .ui(ui);
                    });
                });
            }
        });
    }

    fn show_slider(ui: &mut egui::Ui, range: (f32, f32), step: f32, choice: &mut f32, precision: u8) {
        let range = RangeInclusive::new(
            f32_with_precision(range.0, precision),
            f32_with_precision(range.1, precision),
        );

        ui.horizontal_centered(|ui| {
            ui.spacing_mut().slider_width = 400.0;

            ui.add_space(560.0);
            Slider::new(choice, range)
                .max_decimals(precision as usize)
                .step_by(step as f64)
                .clamp_to_range(true)
                .ui(ui);
        });
    }
}

impl StatefulQItem {
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
