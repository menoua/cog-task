use super::Server;
use eframe::egui;

impl Server {
    pub(crate) fn show_activity(&mut self, _ctx: &egui::Context) {
        /*
        if let Some(scheduler) = self.scheduler.as_ref() {
            match scheduler.view(self.scale_factor) {
                Ok(view) => Column::new()
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_items(Alignment::Center)
                    .push(view)
                    .into(),
                Err(e) => {
                    #[cfg(debug_assertions)]
                    println!("View error: {e:#?}");
                    panic!("Error encountered during view call:\n{e:#?}");
                }
            }
        } else {
            Column::new().into()
        }
         */
    }
}
