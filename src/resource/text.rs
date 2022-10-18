use regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]

pub enum Justification {
    Left,
    Center,
    Right,
}

// impl From<Justification> for Horizontal {
//     fn from(j: Justification) -> Self {
//         match j {
//             Justification::Left => Horizontal::Left,
//             Justification::Center => Horizontal::Center,
//             Justification::Right => Horizontal::Right,
//         }
//     }
// }

pub fn text_or_file(text: &str) -> Option<PathBuf> {
    let re = regex::Regex::new("^!!\\[(.*)\\]$").unwrap();
    if let Some(m) = re.captures(text) {
        if let Ok(path) = PathBuf::from_str(&m[1]) {
            return Some(path);
        }
    }

    None
}
