use cog_task::launcher::Launcher;
use iced::pure::Application;
use iced::{window, Settings};

// const DEFAULT_STYLE: &[u8] = include_bytes!("../assets/launcher.css");

fn main() -> iced::Result {
    // let style = DEFAULT_STYLE.to_vec();
    // if let Some(custom_style) = model.style() {
    //     style.extend(&custom_style);
    // }

    let window_size = Launcher::default().window_size();

    Launcher::run(Settings {
        window: window::Settings {
            size: window_size,
            min_size: Some(window_size),
            max_size: Some(window_size),
            resizable: false,
            decorations: true,
            always_on_top: false,
            icon: None,
            ..Default::default()
        },
        // default_font: None,
        // default_text_size: 0,
        // text_multithreading: false,
        // antialiasing: false,
        // exit_on_close_request: false,
        // try_opengles_first: false,
        ..Default::default()
    })
}
