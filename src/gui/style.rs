use crate::assets::{FONT_ICONS_BRANDS, FONT_ICONS_REGULAR, FONT_ICONS_SOLID};
use crate::util::f32_with_precision;
use eframe::egui;
use eframe::egui::{FontId, TextStyle};
use egui::{Color32, FontData, FontDefinitions, FontFamily, Rgba, Rounding, Stroke, Vec2};
use std::convert::Into;
use std::time::{Duration, Instant};

pub const SCREEN_SIZE: Vec2 = Vec2::new(1920.0, 1080.0);

pub const TEXT_SIZE_HEADING: f32 = 42.0;
pub const TEXT_SIZE_BODY: f32 = 34.0;
pub const TEXT_SIZE_MONOSPACE: f32 = 28.0;
pub const TEXT_SIZE_BUTTON1: f32 = 38.0;
pub const TEXT_SIZE_BUTTON2: f32 = 34.0;
pub const TEXT_SIZE_TOOLTIP: f32 = 20.0;
pub const TEXT_SIZE_ICON: f32 = 32.0;
pub const TEXT_SIZE_DIALOGUE_TITLE: f32 = 30.0;
pub const TEXT_SIZE_DIALOGUE_BODY: f32 = 26.0;

pub const HOVERED: Rgba = Rgba::from_rgb(
    0x67 as f32 / 255.0,
    0x7B as f32 / 255.0,
    0xC4 as f32 / 255.0,
);

pub const FOREST_GREEN: Rgba = Rgba::from_rgb(
    0x22 as f32 / 255.0,
    0x8B as f32 / 255.0,
    0x22 as f32 / 255.0,
);

pub const CUSTOM_RED: Rgba = Rgba::from_rgb(
    0xC0 as f32 / 255.0,
    0x1C as f32 / 255.0,
    0x1C as f32 / 255.0,
);

pub const CUSTOM_ORANGE: Rgba = Rgba::from_rgb(
    0xFF as f32 / 255.0,
    0x6D as f32 / 255.0,
    0x0A as f32 / 255.0,
);

pub const CUSTOM_BLUE: Rgba = Rgba::from_rgb(
    0x00 as f32 / 255.0,
    0x33 as f32 / 255.0,
    0x66 as f32 / 255.0,
);

pub const ACTIVE_BLUE: Rgba = Rgba::from_rgb(
    0x72 as f32 / 255.0,
    0x89 as f32 / 255.0,
    0xDA as f32 / 255.0,
);

pub enum Style {
    IconControls,
    SelectButton,
    CancelButton,
    SubmitButton,
    TodoButton,
    DoneButton,
    InterruptedButton,
    FailedButton,
    SoftFailedButton,
    SingleLineTextEdit,
}

pub fn style_ui(ui: &mut egui::Ui, style: Style) {
    match style {
        Style::IconControls => {
            let rounding = Rounding::same(30.0);
            ui.spacing_mut().item_spacing = Vec2::splat(10.0);
            ui.spacing_mut().button_padding = Vec2::splat(10.0);
            ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
            ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::none();
            ui.visuals_mut().widgets.inactive.rounding = rounding;
            ui.visuals_mut().widgets.hovered.bg_fill =
                Rgba::from(Color32::LIGHT_GRAY).multiply(0.2).into();
            ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::new(1.5, Color32::LIGHT_GRAY);
            ui.visuals_mut().widgets.hovered.rounding = rounding;
            ui.visuals_mut().widgets.active.bg_fill =
                Rgba::from(Color32::LIGHT_GRAY).multiply(0.4).into();
            ui.visuals_mut().widgets.active.bg_stroke = Stroke::new(3.0, Color32::DARK_GRAY);
            ui.visuals_mut().widgets.active.rounding = rounding;
            ui.visuals_mut().widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
            ui.visuals_mut().widgets.noninteractive.bg_stroke = Stroke::none();
            ui.visuals_mut().widgets.noninteractive.rounding = rounding;
        }
        Style::SelectButton => {
            let rounding = Rounding::same(40.0);
            ui.spacing_mut().item_spacing = Vec2::splat(10.0);
            ui.spacing_mut().button_padding = Vec2::new(20.0, 8.0);
            ui.visuals_mut().widgets.inactive.rounding = rounding;
            ui.visuals_mut().widgets.inactive.bg_fill =
                Rgba::from(Color32::LIGHT_GRAY).multiply(0.2).into();
            ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::new(2.0, Color32::LIGHT_GRAY);
            ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(2.0, Color32::BLACK);
            ui.visuals_mut().widgets.hovered.rounding = rounding;
            ui.visuals_mut().widgets.hovered.bg_fill = HOVERED.multiply(0.2).into();
            ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::new(3.0, Color32::LIGHT_GRAY);
            ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(2.0, Color32::BLACK);
            ui.visuals_mut().widgets.active.rounding = rounding;
            ui.visuals_mut().widgets.active.bg_fill = HOVERED.multiply(0.2).into();
            ui.visuals_mut().widgets.active.bg_stroke = Stroke::new(4.0, Color32::DARK_GRAY);
            ui.visuals_mut().widgets.active.fg_stroke = Stroke::new(2.0, Color32::WHITE);
            ui.visuals_mut().widgets.noninteractive.rounding = rounding;
        }
        Style::CancelButton => {
            let rounding = Rounding::same(50.0);
            ui.spacing_mut().button_padding = Vec2::new(60.0, 20.0);
            ui.visuals_mut().widgets.inactive.rounding = rounding;
            ui.visuals_mut().widgets.inactive.bg_fill =
                Rgba::from(Color32::LIGHT_GRAY).multiply(0.2).into();
            ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::new(2.0, Color32::LIGHT_GRAY);
            ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(2.0, CUSTOM_RED);
            ui.visuals_mut().widgets.hovered.rounding = rounding;
            ui.visuals_mut().widgets.hovered.bg_fill =
                Rgba::from(Color32::LIGHT_GRAY).multiply(0.4).into();
            ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::new(3.0, Color32::LIGHT_GRAY);
            ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(2.0, CUSTOM_RED);
            ui.visuals_mut().widgets.active.rounding = rounding;
            ui.visuals_mut().widgets.active.bg_fill =
                Rgba::from(Color32::LIGHT_GRAY).multiply(0.4).into();
            ui.visuals_mut().widgets.active.bg_stroke = Stroke::new(4.0, CUSTOM_RED);
            ui.visuals_mut().widgets.active.fg_stroke = Stroke::new(2.0, CUSTOM_RED);
            ui.visuals_mut().widgets.noninteractive.rounding = rounding;
            ui.visuals_mut().override_text_color = Some(CUSTOM_RED.into());
        }
        Style::SubmitButton => {
            let rounding = Rounding::same(50.0);
            ui.spacing_mut().button_padding = Vec2::new(60.0, 20.0);
            ui.visuals_mut().widgets.inactive.rounding = rounding;
            ui.visuals_mut().widgets.inactive.bg_fill =
                Rgba::from(Color32::LIGHT_GRAY).multiply(0.2).into();
            ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::new(2.0, Color32::LIGHT_GRAY);
            ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(2.0, FOREST_GREEN);
            ui.visuals_mut().widgets.hovered.rounding = rounding;
            ui.visuals_mut().widgets.hovered.bg_fill =
                Rgba::from(Color32::LIGHT_GRAY).multiply(0.4).into();
            ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::new(3.0, Color32::LIGHT_GRAY);
            ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(2.0, FOREST_GREEN);
            ui.visuals_mut().widgets.active.rounding = rounding;
            ui.visuals_mut().widgets.active.bg_fill =
                Rgba::from(Color32::LIGHT_GRAY).multiply(0.4).into();
            ui.visuals_mut().widgets.active.bg_stroke = Stroke::new(4.0, FOREST_GREEN);
            ui.visuals_mut().widgets.active.fg_stroke = Stroke::new(2.0, FOREST_GREEN);
            ui.visuals_mut().widgets.noninteractive.rounding = rounding;
            ui.visuals_mut().override_text_color = Some(FOREST_GREEN.into());
        }
        Style::SingleLineTextEdit => {
            let rounding = Rounding::same(8.0);
            ui.visuals_mut().widgets.inactive.rounding = rounding;
            ui.visuals_mut().widgets.hovered.rounding = rounding;
            ui.visuals_mut().widgets.active.rounding = rounding;
            ui.visuals_mut().widgets.noninteractive.rounding = rounding;
        }
        Style::TodoButton => {
            let rounding = Rounding::same(40.0);
            ui.spacing_mut().button_padding = Vec2::new(30.0, 15.0);
            ui.visuals_mut().widgets.inactive.rounding = rounding;
            ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
            ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::new(2.0, Color32::LIGHT_GRAY);
            ui.visuals_mut().widgets.hovered.rounding = rounding;
            ui.visuals_mut().widgets.hovered.bg_fill =
                Rgba::from(Color32::LIGHT_GRAY).multiply(0.2).into();
            ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::new(3.0, Color32::LIGHT_GRAY);
            ui.visuals_mut().widgets.active.rounding = rounding;
            ui.visuals_mut().widgets.active.bg_fill =
                Rgba::from(Color32::LIGHT_GRAY).multiply(0.4).into();
            ui.visuals_mut().widgets.active.bg_stroke = Stroke::new(4.0, ACTIVE_BLUE);
            ui.visuals_mut().widgets.noninteractive.rounding = rounding;
        }
        Style::DoneButton => {
            let rounding = Rounding::same(40.0);
            ui.spacing_mut().button_padding = Vec2::new(30.0, 15.0);
            ui.visuals_mut().widgets.inactive.rounding = rounding;
            ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
            ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::none();
            ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(2.0, Color32::LIGHT_GRAY);
            ui.visuals_mut().widgets.hovered.rounding = rounding;
            ui.visuals_mut().widgets.hovered.bg_fill =
                Rgba::from(Color32::LIGHT_GRAY).multiply(0.2).into();
            ui.visuals_mut().widgets.hovered.bg_stroke = Stroke::none();
            ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(4.0, Color32::LIGHT_GRAY);
            ui.visuals_mut().widgets.active.rounding = rounding;
            ui.visuals_mut().widgets.active.bg_fill =
                Rgba::from(Color32::LIGHT_GRAY).multiply(0.4).into();
            ui.visuals_mut().widgets.active.bg_stroke = Stroke::new(4.0, ACTIVE_BLUE);
            ui.visuals_mut().widgets.noninteractive.rounding = rounding;
            ui.visuals_mut().override_text_color = Some(FOREST_GREEN.into());
        }
        Style::InterruptedButton => {
            style_ui(ui, Style::TodoButton);
            ui.visuals_mut().override_text_color = Some(CUSTOM_BLUE.into());
        }
        Style::FailedButton => {
            style_ui(ui, Style::TodoButton);
            ui.visuals_mut().override_text_color = Some(CUSTOM_RED.into());
        }
        Style::SoftFailedButton => {
            style_ui(ui, Style::TodoButton);
            ui.visuals_mut().override_text_color = Some(CUSTOM_ORANGE.into());
        }
    }
}

pub fn init(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = FontDefinitions::default();

    // Icon fonts from font-awesome
    fonts.font_data.insert(
        "fa_brands_regular".to_owned(),
        FontData::from_static(FONT_ICONS_BRANDS),
    );
    fonts.font_data.insert(
        "fa_free_regular".to_owned(),
        FontData::from_static(FONT_ICONS_REGULAR),
    );
    fonts.font_data.insert(
        "fa_free_solid".to_owned(),
        FontData::from_static(FONT_ICONS_SOLID),
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

    // Redefine text_styles sizes
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(TEXT_SIZE_HEADING, FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(TEXT_SIZE_BODY, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(TEXT_SIZE_MONOSPACE, FontFamily::Proportional),
        ),
        (
            TextStyle::Button,
            FontId::new(TEXT_SIZE_BUTTON2, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(TEXT_SIZE_TOOLTIP, FontFamily::Proportional),
        ),
    ]
    .into();
    ctx.set_style(style);

    let mut visuals = egui::Visuals::light();
    visuals.override_text_color = Some(Color32::BLACK);
    ctx.set_visuals(visuals);
}

pub fn set_fullscreen_scale(ctx: &egui::Context, scale: f32) {
    static mut RESCALE_TIMER: Option<Instant> = None;

    let curr = ctx.pixels_per_point();
    let size = ctx.input().screen_rect().size();
    let mut scale = (size.x / SCREEN_SIZE.x).min(size.y / SCREEN_SIZE.y) * scale;
    scale = f32_with_precision(scale, 6);

    if (scale - 1.0).abs() > 1e-6 {
        let now = Instant::now();
        unsafe {
            match RESCALE_TIMER {
                None => {
                    println!("Rescaling UI {curr}*{scale}={}", curr * scale);
                    RESCALE_TIMER = Some(now);
                }
                Some(timer) if timer.elapsed() > Duration::from_millis(500) => {
                    println!("Rescaling UI {curr}*{scale}={}", curr * scale);
                    RESCALE_TIMER = Some(now);
                }
                _ => scale = 1.0,
            }
        }
    } else {
        scale = 1.0;
    }

    ctx.set_pixels_per_point(curr * scale);
}

pub mod text {
    use super::*;
    use eframe::egui::{Color32, RichText};

    #[inline(always)]
    pub fn heading(text: impl Into<String>) -> RichText {
        RichText::new(text).heading()
    }

    #[inline(always)]
    pub fn body(text: impl Into<String>) -> RichText {
        RichText::new(text).size(TEXT_SIZE_BODY)
    }

    #[inline(always)]
    pub fn inactive(text: impl Into<String>) -> RichText {
        RichText::new(text)
            .size(TEXT_SIZE_BODY)
            .color(Color32::LIGHT_GRAY)
    }

    #[inline(always)]
    pub fn button1(text: impl Into<String>) -> RichText {
        RichText::new(text).size(TEXT_SIZE_BUTTON1)
    }

    #[inline(always)]
    pub fn button2(text: impl Into<String>) -> RichText {
        RichText::new(text).size(TEXT_SIZE_BUTTON2)
    }

    #[inline(always)]
    pub fn tooltip(text: impl Into<String>) -> RichText {
        RichText::new(text).size(TEXT_SIZE_TOOLTIP)
    }

    #[inline(always)]
    pub fn icon(text: impl Into<String>) -> RichText {
        RichText::new(text).size(TEXT_SIZE_ICON)
    }
}
