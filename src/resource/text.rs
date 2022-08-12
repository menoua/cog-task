use iced::alignment::Horizontal;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]

pub enum Justification {
    Left,
    Center,
    Right,
}

impl From<Justification> for Horizontal {
    fn from(j: Justification) -> Self {
        match j {
            Justification::Left => Horizontal::Left,
            Justification::Center => Horizontal::Center,
            Justification::Right => Horizontal::Right,
        }
    }
}

pub fn text_or_file(text: &str) -> Option<PathBuf> {
    let parts: Vec<_> = text.split_whitespace().collect();
    if parts.len() == 2 && parts[0] == "<" {
        Some(PathBuf::from_str(parts[1]).unwrap())
    } else {
        None
    }
}
