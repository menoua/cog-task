use crate::server::{Server, ServerSignal};
use eframe::egui;
use eyre::eyre;

impl Server {
    #[inline]
    pub(crate) fn show_activity(&mut self, ui: &mut egui::Ui) {
        if let Some(scheduler) = self.scheduler.as_mut() {
            if let Err(e) = scheduler.show(ui) {
                self.sync_reader.push(ServerSignal::BlockCrashed(e));
            }
        } else {
            self.sync_reader.push(ServerSignal::BlockCrashed(eyre!(
                "Scheduler died while a task was active."
            )));
        }
    }
}
