use eframe::egui;

#[derive(Debug)]
pub enum Monitor {
    Keys,
}

#[derive(Debug, Clone)]
pub enum Event {
    Key(egui::Key),
}
