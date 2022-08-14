use iced::keyboard::KeyCode;

#[derive(Debug)]
pub enum Monitor {
    Keys,
    Frames(f64),
}

#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyCode),
    Refresh,
}
