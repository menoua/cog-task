use super::Server;
use eframe::egui;

impl Server {
    #[inline]
    pub(crate) fn show_cleanup(&mut self, ui: &mut egui::Ui) {
        ui.centered_and_justified(|ui| {
            ui.heading("...");
        });
    }
}
