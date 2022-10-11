use super::Server;
use eframe::egui;
use eframe::egui::CursorIcon;
use egui::{CentralPanel, Color32, RichText};

impl Server {
    pub(crate) fn show_loading(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.output().cursor_icon = CursorIcon::None;

            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("...").color(Color32::BLACK).heading());
            })
        });
    }
}
