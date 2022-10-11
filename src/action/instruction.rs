use crate::action::{Action, StatefulAction, StatefulActionMsg};
use crate::callback::{CallbackQueue, Destination};
use crate::config::Config;
use crate::error::Error::TaskDefinitionError;
use crate::io::IO;
use crate::resource::text::Justification;
use crate::resource::{text::text_or_file, ResourceMap};
use crate::scheduler::{AsyncCallback, SyncCallback};
use crate::style::text::body;
use crate::style::{style_ui, Style};
use crate::template::header_body_controls;
use crate::{error, style};
use eframe::egui;
use eframe::egui::{CentralPanel, RichText, ScrollArea};
use eframe::epaint::Color32;
use egui_extras::{Size, StripBuilder};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
        id: usize,
        res: &ResourceMap,
        _config: &Config,
        _io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
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

        Ok(Box::new(StatefulInstruction {
            id,
            done: false,
            text,
            header,
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
    header: String,
    justify: Justification,
    persistent: bool,
}

impl StatefulAction for StatefulInstruction {
    #[inline(always)]
    fn id(&self) -> usize {
        self.id
    }

    #[inline(always)]
    fn is_over(&self) -> Result<bool, error::Error> {
        Ok(self.done)
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
    fn stop(&mut self) -> Result<(), error::Error> {
        self.done = true;
        Ok(())
    }

    fn show(
        &mut self,
        ctx: &egui::Context,
        sync_queue: &mut CallbackQueue<SyncCallback>,
        async_queue: &mut CallbackQueue<AsyncCallback>,
    ) -> Result<(), error::Error> {
        CentralPanel::default().show(ctx, |ui| {
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
                strip.strip(|builder| self.show_controls(builder, sync_queue));
            });
        });

        Ok(())
    }
}

impl StatefulInstruction {
    fn show_controls(
        &mut self,
        builder: StripBuilder,
        sync_queue: &mut CallbackQueue<SyncCallback>,
    ) {
        enum Interaction {
            None,
            Next,
        }

        let mut interaction = Interaction::None;

        builder
            .size(Size::remainder())
            .size(Size::exact(200.0))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.empty();

                strip.cell(|ui| {
                    ui.horizontal_centered(|ui| {
                        style_ui(ui, Style::SubmitButton);
                        if ui.button(RichText::new("Next").size(40.0)).clicked() {
                            interaction = Interaction::Next;
                        }
                    });
                });

                strip.empty();
            });

        match interaction {
            Interaction::None => {}
            Interaction::Next => {
                self.done = true;
                sync_queue.push(Destination::default(), SyncCallback::UpdateGraph);
            }
        }
    }
}
