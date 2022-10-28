use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE, VISUAL};
use crate::comm::{QWriter, Signal, SignalId};
use crate::gui::{center_x, header_body_controls, style_ui, text::button1, Style};
use crate::resource::{parse_text, text_or_file, ResourceAddr, ResourceMap, IO};
use crate::server::{AsyncSignal, Config, State, SyncSignal};
use crate::util::f64_as_i64;
use eframe::egui;
use eframe::egui::{CursorIcon, ScrollArea};
use egui_extras::{Size, StripBuilder};
use eyre::{eyre, Error, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_cbor::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Instruction {
    text: String,
    #[serde(default)]
    header: String,
    #[serde(default)]
    params: BTreeMap<String, String>,
    #[serde(default)]
    in_mapping: BTreeMap<SignalId, String>,
    #[serde(default = "defaults::persistent")]
    #[serde(rename = "static")]
    persistent: bool,
}

stateful!(Instruction {
    text: String,
    header: String,
    params: BTreeMap<String, String>,
    persistent: bool,
    in_mapping: BTreeMap<SignalId, String>,
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
            params: BTreeMap::new(),
            persistent: defaults::persistent(),
            in_mapping: BTreeMap::new(),
        }
    }
}

impl Action for Instruction {
    #[inline]
    fn in_signals(&self) -> BTreeSet<SignalId> {
        self.in_mapping.keys().cloned().collect()
    }

    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<ResourceAddr> {
        if let Some(path) = text_or_file(&self.text) {
            vec![ResourceAddr::Text(path)]
        } else {
            vec![]
        }
    }

    fn init(mut self) -> Result<Box<dyn Action>, Error> {
        let re = Regex::new(r"\$\{([[:alpha:]][[:word:]]*)\}").unwrap();
        for caps in re.captures_iter(&self.text) {
            self.params
                .entry(caps[1].to_owned())
                .or_insert_with(|| "<UNSET>".to_owned());
        }

        for (_, v) in self.in_mapping.iter() {
            if !self.params.contains_key(v) {
                return Err(eyre!("Undefined parameter `{v}` in `in_mapping`."));
            }
        }

        Ok(Box::new(self))
    }

    fn stateful(
        &self,
        _io: &IO,
        res: &ResourceMap,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        Ok(Box::new(StatefulInstruction {
            done: false,
            text: res.fetch_text(&self.text)?,
            header: self.header.clone(),
            params: self.params.clone(),
            persistent: self.persistent,
            in_mapping: self.in_mapping.clone(),
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
        state: &State,
    ) -> Result<Signal> {
        for (id, key) in self.in_mapping.iter() {
            if let Some(entry) = self.params.get_mut(key) {
                if let Some(value) = state.get(id) {
                    *entry = match value {
                        Value::Bool(v) => v.to_string(),
                        Value::Integer(v) => v.to_string(),
                        Value::Float(v) => format!("{v:.4}"),
                        Value::Text(v) => v.to_string(),
                        Value::Null => "<UNSET>".to_owned(),
                        _ => "<INVALID>".to_owned(),
                    };
                }
            }
        }

        sync_writer.push(SyncSignal::Repaint);
        Ok(Signal::none())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        state: &State,
    ) -> Result<Signal> {
        let mut changed = false;
        if let ActionSignal::StateChanged(_, signal) = signal {
            for id in signal {
                if let Some(key) = self.in_mapping.get(id) {
                    if let Some(entry) = self.params.get_mut(key) {
                        *entry = match state.get(id).unwrap() {
                            Value::Bool(v) => v.to_string(),
                            Value::Integer(v) => v.to_string(),
                            Value::Float(v) => {
                                if let Ok(v) = f64_as_i64(*v) {
                                    v.to_string()
                                } else {
                                    format!("{v:.4}")
                                }
                            }
                            Value::Text(v) => v.to_string(),
                            Value::Null => "<UNSET>".to_owned(),
                            _ => "<INVALID>".to_owned(),
                        };
                    }
                    changed = true;
                }
            }
        }

        if changed {
            sync_writer.push(SyncSignal::Repaint);
        }
        Ok(Signal::none())
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<()> {
        let mut text = self.text.clone();

        for (k, v) in self.params.iter() {
            text = Regex::new(&format!(r"\$\{{{k}}}"))
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
                                    let _ = parse_text(ui, &text);
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
    ) -> Result<Signal> {
        self.done = true;
        sync_writer.push(SyncSignal::Repaint);
        Ok(Signal::none())
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
