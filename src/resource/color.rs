use eframe::egui::Color32;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    Inherit,
    Transparent,
    White,
    Black,
    Gray,
    Red,
    Blue,
    Green,
    Yellow,
    Rgb(u8, u8, u8),
    Rgba(u8, u8, u8, u8),
}

impl Default for Color {
    #[inline(always)]
    fn default() -> Self {
        Color::Inherit
    }
}

impl From<&Color> for Color32 {
    #[inline]
    fn from(c: &Color) -> Self {
        match c {
            Color::Inherit | Color::Transparent => Color32::TRANSPARENT,
            Color::White => Color32::WHITE,
            Color::Black => Color32::BLACK,
            Color::Gray => Color32::GRAY,
            Color::Red => Color32::RED,
            Color::Blue => Color32::BLUE,
            Color::Green => Color32::GREEN,
            Color::Yellow => Color32::YELLOW,
            Color::Rgb(r, g, b) => Color32::from_rgb(*r, *g, *b),
            Color::Rgba(r, g, b, a) => Color32::from_rgba_unmultiplied(*r, *g, *b, *a),
        }
    }
}

impl From<Color> for Color32 {
    #[inline(always)]
    fn from(c: Color) -> Self {
        Self::from(&c)
    }
}

impl Color {
    pub fn or(&self, other: &Self) -> Self {
        if let Self::Inherit = self {
            *other
        } else {
            *self
        }
    }
}
