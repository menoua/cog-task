use iced::alignment::{Horizontal, Vertical};
use iced::{pure::text, pure::Element, Font};
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

pub const FONT_ICONS_BRANDS: Font = Font::External {
    name: "fa-brands",
    bytes: include_bytes!("assets/fonts/fa-6-brands-regular-400.otf"),
};
pub const FONT_ICONS_REGULAR: Font = Font::External {
    name: "fa-regular",
    bytes: include_bytes!("assets/fonts/fa-6-free-regular-400.otf"),
};
pub const FONT_ICONS_SOLID: Font = Font::External {
    name: "fa-solid",
    bytes: include_bytes!("assets/fonts/fa-6-free-solid-900.otf"),
};
pub enum Icon {
    Help,
    SystemInfo,
    Clipboard,
    Close,
    Folder,
    FolderTree,
    MagnifyingGlass,
}
impl<'a, T> From<Icon> for Element<'a, T> {
    fn from(icon: Icon) -> Self {
        match icon {
            Icon::Help => text("\u{f059}").font(FONT_ICONS_REGULAR),
            Icon::SystemInfo => text("\u{f05a}").font(FONT_ICONS_SOLID),
            Icon::Clipboard => text("\u{f328}").font(FONT_ICONS_REGULAR),
            Icon::Close => text("\u{f00d}").font(FONT_ICONS_SOLID),
            Icon::Folder => text("\u{f07b}").font(FONT_ICONS_REGULAR),
            Icon::FolderTree => text("\u{f802}").font(FONT_ICONS_SOLID),
            Icon::MagnifyingGlass => text("\u{f002}").font(FONT_ICONS_SOLID),
        }
        .size(26)
        .vertical_alignment(Vertical::Center)
        .horizontal_alignment(Horizontal::Center)
        .into()
    }
}
