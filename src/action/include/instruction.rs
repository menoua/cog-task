use crate::action::{Action, Props, StatefulAction, VISUAL, ActionEnum, StatefulActionEnum, INFINITE, ActionSignal};
use crate::signal::QWriter;
use crate::config::Config;
use crate::error::Error::TaskDefinitionError;
use crate::io::IO;
use crate::resource::text::Justification;
use crate::resource::{text::text_or_file, ResourceMap};
use crate::style::text::{body, button1};
use crate::style::{style_ui, Style};
use crate::template::{center_x, header_body_controls};
use crate::error;
use eframe::egui;
use eframe::egui::{RichText, ScrollArea};
use egui_extras::{Size, StripBuilder};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::error::Error;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Instruction {
    text: String,
    #[serde(default)]
    header: String,
    #[serde(default = "defaults::justification")]
    justify: String,
    #[serde(default = "defaults::persistent")]
    #[serde(rename = "static")]
    persistent: bool,
    #[serde(default)]
    style: String,
}

stateful!(Instruction {
    text: String,
    header: String,
    justify: Justification,
    persistent: bool,
});

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
            header: "".to_owned(),
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
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<StatefulActionEnum, error::Error> {
        let text = res.fetch_text(&self.text)?;
        let header = self.header.clone();
        let justify = match self.justify.to_lowercase().as_str() {
            "left" => Justification::Left,
            "center" => Justification::Center,
            "right" => Justification::Right,
            j => Err(TaskDefinitionError(format!(
                "Unknown justification value '{j}' (should be 'left', 'center', or 'right')"
            )))?,
        };

        Ok(StatefulInstruction {
            id: 0,
            done: false,
            text,
            header,
            justify,
            persistent: self.persistent,
            // style: Style::new("action-instruction", &self.style),
        }.into())
    }
}

impl StatefulAction for StatefulInstruction {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        if self.persistent {
            INFINITE | VISUAL
        } else {
            VISUAL
        }.into()
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
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        header_body_controls(ui, |mut strip| {
            strip.cell(|ui| {
                ui.centered_and_justified(|ui| ui.heading(&self.header));
            });
            strip.empty();
            strip.strip(|builder| {
                builder
                    .size(Size::remainder())
                    .size(Size::exact(1520.0))
                    .size(Size::remainder())
                    .horizontal(|mut strip| {
                        strip.empty();
                        strip.cell(|ui| {
                            ScrollArea::vertical().show(ui, |ui| {
                                ui.centered_and_justified(|ui| {
                                    ui.label(body(&self.text));
                                });
                            });
                        });
                        strip.empty();
                    });
            });
            strip.empty();
            strip.strip(|builder| self.show_controls(builder, sync_writer));
        });

        Ok(())
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        self.done = true;
        Ok(())
    }
}

impl StatefulInstruction {
    fn show_controls(
        &mut self,
        builder: StripBuilder,
        sync_writer: &mut QWriter<SyncSignal>,
    ) {
        enum Interaction {
            None,
            Next,
        }

        let mut interaction = Interaction::None;

        center_x(builder, 200.0, |ui| {
            ui.horizontal_centered(|ui| {
                style_ui(ui, Style::SubmitButton);
                if ui.button(button1("Next")).clicked() {
                    interaction = Interaction::Next;
                }
            });
        });

        match interaction {
            Interaction::None => {}
            Interaction::Next => {
                self.done = true;
                sync_writer.push(SyncSignal::UpdateGraph);
            }
        }
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("persistent", format!("{:?}", self.persistent))])
            .collect()
    }
}
