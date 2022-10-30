use crate::gui::text::body;
use eframe::egui::Ui;
use egui_demo_lib::easy_mark::easy_mark;
use eyre::{eyre, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn parse_text(ui: &mut Ui, text: &str) -> Result<()> {
    let re = Regex::new(r"^!!<([[:alpha:]][[:word:]]*)>[ \t]*\n?([ \t]*\n)?").unwrap();
    if let Some(caps) = re.captures(text) {
        match &caps[1] {
            "easy_mark" => {
                easy_mark(ui, &re.replace(text, ""));
                Ok(())
            }
            parser => Err(eyre!("Unknown text parser ({parser}).")),
        }
    } else {
        ui.label(body(text));
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OptionalString {
    Some(String),
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OptionalPath {
    Some(PathBuf),
    None,
}

impl Default for OptionalString {
    fn default() -> Self {
        Self::None
    }
}

impl OptionalString {
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl Default for OptionalPath {
    fn default() -> Self {
        Self::None
    }
}

impl OptionalPath {
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}
