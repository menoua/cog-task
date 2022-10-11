use crate::server::Server;
use crate::style::text::{body, heading};
use eframe::egui;
use eframe::egui::{RichText, ScrollArea};
use egui_extras::{Size, Strip, StripBuilder};

pub fn header_body_controls(ui: &mut egui::Ui, content: impl FnOnce(&mut Strip)) {
    StripBuilder::new(ui)
        .size(Size::exact(25.0))
        .size(Size::exact(100.0))
        .size(Size::exact(30.0))
        .size(Size::remainder())
        .size(Size::exact(30.0))
        .size(Size::exact(100.0))
        .size(Size::exact(25.0))
        .vertical(|mut strip| {
            strip.empty();
            content(&mut strip);
            strip.empty();
        });
}
