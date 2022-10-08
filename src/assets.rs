use eframe::egui;
use eframe::egui::{FontData, FontDefinitions, FontFamily, FontId, RichText, WidgetText};
use iced::alignment::{Horizontal, Vertical};
use iced::{pure::text, pure::Element, Font};
use spin_sleep::SpinStrategy;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const PIXELS_PER_POINT: f32 = 4.0;

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
            Icon::Help => text("\u{f059}").font(FONT_ICONS_SOLID),
            Icon::SystemInfo => text("\u{f05a}").font(FONT_ICONS_SOLID),
            Icon::Clipboard => text("\u{f328}").font(FONT_ICONS_REGULAR),
            Icon::Close => text("\u{f00d}").font(FONT_ICONS_SOLID),
            Icon::Folder => text("\u{f07b}").font(FONT_ICONS_SOLID),
            Icon::FolderTree => text("\u{f802}").font(FONT_ICONS_SOLID),
            Icon::MagnifyingGlass => text("\u{f002}").font(FONT_ICONS_SOLID),
        }
        .size(26)
        .vertical_alignment(Vertical::Center)
        .horizontal_alignment(Horizontal::Center)
        .into()
    }
}
impl From<Icon> for WidgetText {
    fn from(icon: Icon) -> Self {
        RichText::from(match icon {
            Icon::Help => "\u{f059}",
            Icon::SystemInfo => "\u{f05a}",
            Icon::Clipboard => "\u{f328}",
            Icon::Close => "\u{f00d}",
            Icon::Folder => "\u{f07b}",
            Icon::FolderTree => "\u{f802}",
            Icon::MagnifyingGlass => "\u{f002}",
        })
        .font(FontId::new(10.0, FontFamily::Name("fa_free".into())))
        .into()
    }
}

pub fn setup(ctx: &egui::Context) {
    ctx.set_pixels_per_point(PIXELS_PER_POINT);

    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = FontDefinitions::default();

    // Icon fonts from font-awesome
    fonts.font_data.insert(
        "fa_brands_regular".to_owned(),
        FontData::from_static(include_bytes!("assets/fonts/fa-6-brands-regular-400.otf")),
    );
    fonts.font_data.insert(
        "fa_free_regular".to_owned(),
        FontData::from_static(include_bytes!("assets/fonts/fa-6-free-regular-400.otf")),
    );
    fonts.font_data.insert(
        "fa_free_solid".to_owned(),
        FontData::from_static(include_bytes!("assets/fonts/fa-6-free-solid-900.otf")),
    );
    fonts
        .families
        .entry(FontFamily::Name("fa_free".into()))
        .or_default()
        .extend(vec![
            "fa_free_regular".to_owned(),
            "fa_free_solid".to_owned(),
            "fa_brands_regular".to_owned(),
        ]);

    // // Put my font first (highest priority) for proportional text:
    // fonts
    //     .families
    //     .entry(egui::FontFamily::Proportional)
    //     .or_default()
    //     .insert(0, "my_font".to_owned());

    // // Put my font as last fallback for monospace:
    // fonts
    //     .families
    //     .entry(egui::FontFamily::Monospace)
    //     .or_default()
    //     .push("my_font".to_owned());

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}
