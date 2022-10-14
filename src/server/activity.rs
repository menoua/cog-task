use super::Server;
use crate::error::Error::InternalError;
use crate::server::ServerCallback;
use eframe::egui;
use eframe::egui::{CentralPanel, Color32, RichText};

impl Server {
    pub(crate) fn show_activity(&mut self, ui: &mut egui::Ui) {
        if let Some(scheduler) = self.scheduler.as_mut() {
            if let Err(e) = scheduler.show(ui) {
                self.sync_qr.push(ServerCallback::BlockCrashed(e));
            }
        } else {
            self.sync_qr
                .push(ServerCallback::BlockCrashed(InternalError(
                    "Unexpected behavior: Scheduler died while a task was active!".to_owned(),
                )));
        }
    }
}
