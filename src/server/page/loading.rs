use crate::server::Server;
use eframe::egui;
use eframe::egui::CursorIcon;

impl Server {
    #[inline]
    pub(crate) fn show_loading(&mut self, ui: &mut egui::Ui) {
        ui.output().cursor_icon = CursorIcon::None;

        ui.centered_and_justified(|ui| {
            ui.heading("...");
        });
    }
}
