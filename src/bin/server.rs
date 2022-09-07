use cog_task::assets::VERSION;
use cog_task::server::Server;
use iced::pure::Application;
use iced::{window, Error, Settings};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn main() -> Result<(), Error> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        println!("Invalid number of arguments. Correct usage:\n./server path_to_task_dir");
        std::process::exit(1);
    } else {
        println!("Starting task \"{}\" with Server-v{}...", args[1], VERSION);
    }

    let bin = PathBuf::from(&args[0]);
    let mut hasher = Sha256::default();
    hasher.update(&std::fs::read(bin).unwrap());
    let bin_hash = hex::encode(hasher.finalize());

    let path = PathBuf::from(&args[1]);

    Server::run(Settings {
        window: window::Settings {
            size: (1000, 700),
            min_size: Some((1000, 700)),
            resizable: true,
            decorations: false,
            always_on_top: true,
            icon: None,
            ..Default::default()
        },
        flags: (path, bin_hash),
        // default_font: None,
        // default_text_size: 0,
        // text_multithreading: false,
        // antialiasing: false,
        exit_on_close_request: false,
        // try_opengles_first: false,
        ..Default::default()
    })
}
