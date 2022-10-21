use super::Server;
use crate::error::Error::InternalError;
use crate::server::ServerSignal;
use eframe::egui;

impl Server {
    #[inline]
    pub(crate) fn show_activity(&mut self, ui: &mut egui::Ui) {
        if let Some(scheduler) = self.scheduler.as_mut() {
            if let Err(e) = scheduler.show(ui) {
                self.sync_reader.push(ServerSignal::BlockCrashed(e));
            }
        } else {
            self.sync_reader
                .push(ServerSignal::BlockCrashed(InternalError(
                    "Unexpected behavior: Scheduler died while a task was active!".to_owned(),
                )));
        }
    }
}
