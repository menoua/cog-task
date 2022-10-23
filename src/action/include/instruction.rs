use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE, VISUAL};
use crate::config::Config;
use crate::error;
use crate::error::Error;
use crate::error::Error::TaskDefinitionError;
use crate::io::IO;
use crate::queue::QWriter;
use crate::resource::{text::text_or_file, ResourceMap};
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::scheduler::State;
use crate::signal::SignalId;
use crate::style::text::{body, button1};
use crate::style::{style_ui, Style};
use crate::template::{center_x, header_body_controls};
use eframe::egui;
use eframe::egui::{CursorIcon, ScrollArea};
use egui_extras::{Size, StripBuilder};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Instruction {
    text: String,
    #[serde(default)]
    header: String,
    #[serde(default)]
    params: HashMap<SignalId, String>,
    #[serde(default = "defaults::persistent")]
    #[serde(rename = "static")]
    persistent: bool,
}

stateful!(Instruction {
    text: String,
    header: String,
    params_i: HashMap<u16, String>,
    params_e: HashMap<u16, String>,
    params_s: HashSet<u16>,
    persistent: bool,
});

mod defaults {
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
            params: HashMap::new(),
            persistent: defaults::persistent(),
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

    fn init(mut self) -> Result<Box<dyn Action>, Error> {
        let re = Regex::new(r"#([ies])\(((0x)?\d+)\)").unwrap();
        for caps in re.captures_iter(&self.text) {
            let key = if caps[2].starts_with("0x") {
                u16::from_str_radix(caps[2].trim_start_matches("0x"), 16).map_err(|_| {
                    TaskDefinitionError(format!(
                        "Failed to parse hexadecimal integer: {}",
                        &caps[2]
                    ))
                })
            } else {
                caps[2].parse::<u16>().map_err(|_| {
                    TaskDefinitionError(format!("Failed to parse decimal integer: {}", &caps[2]))
                })
            }?;

            let key = match &caps[1] {
                "i" => Ok(SignalId::Internal(key)),
                "e" => Ok(SignalId::External(key)),
                "s" => Ok(SignalId::State(key)),
                _ => Err(TaskDefinitionError(format!(
                    "Unknown signal identifier: {}",
                    &caps[1]
                ))),
            }?;

            if !self.params.contains_key(&key) {
                self.params.insert(key, "<UNSET>".to_owned());
            }
        }

        self.text = re
            .replace_all(&self.text, |caps: &Captures| {
                format!(
                    "#{}({})",
                    &caps[1],
                    if caps[2].starts_with("0x") {
                        u16::from_str_radix(&caps[2].trim_start_matches("0x"), 16).unwrap()
                    } else {
                        caps[2].parse::<u16>().unwrap()
                    }
                )
            })
            .parse()
            .unwrap();

        Ok(Box::new(self))
    }

    fn stateful(
        &self,
        _io: &IO,
        res: &ResourceMap,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        let mut params_i = HashMap::new();
        let mut params_e = HashMap::new();
        let mut params_s = HashSet::new();
        for (k, v) in self.params.iter() {
            match k {
                SignalId::None => {}
                SignalId::Internal(i) => {
                    params_i.insert(*i, v.clone());
                }
                SignalId::External(i) => {
                    params_e.insert(*i, v.clone());
                }
                SignalId::State(i) => {
                    params_s.insert(*i);
                }
            };
        }

        Ok(Box::new(StatefulInstruction {
            done: false,
            text: res.fetch_text(&self.text)?,
            header: self.header.clone(),
            params_i,
            params_e,
            params_s,
            persistent: self.persistent,
        }))
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
        }
        .into()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        sync_writer.push(SyncSignal::Repaint);
        Ok(())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        match signal {
            ActionSignal::Internal(_, signal) => {
                for (k, v) in signal.iter() {
                    if let Some(entry) = self.params_i.get_mut(k) {
                        *entry = match v {
                            Value::Bool(v) => format!("{v}"),
                            Value::Integer(v) => format!("{v}"),
                            Value::Float(v) => format!("{v:.4}"),
                            Value::Text(v) => format!("{v}"),
                            Value::Null => "<UNSET>".to_owned(),
                            _ => "<INVALID>".to_owned(),
                        };
                    }
                }
            }
            ActionSignal::StateChanged => {
                if !self.params_s.is_empty() {
                    sync_writer.push(SyncSignal::Repaint);
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<(), error::Error> {
        let mut text = self.text.clone();

        for (i, v) in self.params_i.iter() {
            text = Regex::new(&format!(r"#i\({i}\)"))
                .unwrap()
                .replace_all(&text, v)
                .to_string();
        }
        for (i, v) in self.params_e.iter() {
            text = Regex::new(&format!(r"#e\({i}\)"))
                .unwrap()
                .replace_all(&text, v)
                .to_string();
        }
        for i in self.params_s.iter() {
            let v = match state.get(i).unwrap_or(&Value::Null) {
                Value::Bool(v) => format!("{v}"),
                Value::Integer(v) => format!("{v}"),
                Value::Float(v) => format!("{v:.4}"),
                Value::Text(v) => format!("{v}"),
                Value::Null => "<UNSET>".to_owned(),
                _ => "<INVALID>".to_owned(),
            };

            text = Regex::new(&format!(r"#s\({i}\)"))
                .unwrap()
                .replace_all(&text, v)
                .to_string();
        }

        header_body_controls(ui, |strip| {
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
                                    ui.label(body(&text));
                                });
                            });
                        });
                        strip.empty();
                    });
            });
            strip.empty();
            strip.strip(|builder| {
                if !self.persistent {
                    self.show_controls(builder, sync_writer);
                }
            });
        });

        if self.persistent {
            ui.output().cursor_icon = CursorIcon::None;
        }

        Ok(())
    }

    #[inline(always)]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), error::Error> {
        self.done = true;
        sync_writer.push(SyncSignal::Repaint);
        Ok(())
    }
}

impl StatefulInstruction {
    fn show_controls(&mut self, builder: StripBuilder, sync_writer: &mut QWriter<SyncSignal>) {
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

    #[allow(dead_code)]
    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("persistent", format!("{:?}", self.persistent))])
            .collect()
    }
}
