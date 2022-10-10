use super::Server;
use eframe::egui;
use egui::{CentralPanel, Color32, RichText};

impl Server {
    pub(crate) fn show_cleanup(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("...").color(Color32::BLACK).heading());
            })
        });
    }
}
