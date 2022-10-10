use super::Server;
use eframe::egui;
use eframe::egui::CentralPanel;

impl Server {
    pub(crate) fn show_activity(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            if let Some(scheduler) = self.scheduler.as_ref() {
                ui.centered_and_justified(|ui| ui.heading("[Activity goes here...]"));
            } else {
                ui.centered_and_justified(|ui| ui.heading("[SCHEDULER IS MISSING!]"));
            }
        });
    }
}
