use crate::style::TEXT_SIZE_ICON;
use eframe::egui;
use eframe::egui::{FontData, FontDefinitions, FontFamily, FontId, RichText, WidgetText};
use spin_sleep::SpinStrategy;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const TEXT_TITLE: u16 = 45;
pub const TEXT_XLARGE: u16 = 40;
pub const TEXT_LARGE: u16 = 36;
pub const TEXT_NORMAL: u16 = 34;
pub const TEXT_SMALL: u16 = 32;
pub const TEXT_XSMALL: u16 = 28;
pub const TEXT_TINY: u16 = 24;

pub const SPIN_DURATION: u32 = 100_000_000; // equivalent to 100ms
pub const SPIN_STRATEGY: SpinStrategy = SpinStrategy::SpinLoopHint;

pub const IMAGE_FIXATION: &[u8] = include_bytes!("assets/fixation.svg");
pub const IMAGE_RUSTACEAN: &[u8] = include_bytes!("assets/rustacean.svg");

pub const FONT_ICONS_BRANDS: &[u8] = include_bytes!("assets/fonts/fa-6-brands-regular-400.otf");
pub const FONT_ICONS_REGULAR: &[u8] = include_bytes!("assets/fonts/fa-6-free-regular-400.otf");
pub const FONT_ICONS_SOLID: &[u8] = include_bytes!("assets/fonts/fa-6-free-solid-900.otf");

pub enum Icon {
    Help,
    SystemInfo,
    Clipboard,
    Close,
    Folder,
    FolderTree,
    MagnifyingGlass,
}
impl Icon {
    pub fn size(self, size: f32) -> RichText {
        RichText::from(self).size(size)
    }
}
impl From<Icon> for RichText {
    fn from(icon: Icon) -> Self {
        RichText::new(match icon {
            Icon::Help => "\u{f059}",
            Icon::SystemInfo => "\u{f05a}",
            Icon::Clipboard => "\u{f328}",
            Icon::Close => "\u{f00d}",
            Icon::Folder => "\u{f07b}",
            Icon::FolderTree => "\u{f802}",
            Icon::MagnifyingGlass => "\u{f002}",
        })
        .font(FontId::new(
            TEXT_SIZE_ICON,
            FontFamily::Name("fa_free".into()),
        ))
    }
}
impl From<Icon> for WidgetText {
    fn from(icon: Icon) -> Self {
        RichText::from(icon).into()
    }
}
