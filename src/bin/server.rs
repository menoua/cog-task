// todo!("Maybe come up with a way to allow multiple µComponent-type actions to \
//        run simultaneously in special circumstances; think if it is even useful;\
//        for example maybe allow 2 horizontal (or vertical) panes instead of single
//        viewport");

// todo!("Add cross-fade support by providing fade-in/out duration (global, block, and local \
//        -- like how gain is set)")

// todo!("Add Compound action functionality, which uses 2 NOPs -- one before the sub-origin \
//        set, and one after all sub-actions end")

// todo!("Add volume option to video")

use cog_task::assets::VERSION;
use cog_task::server::Server;
use iced::pure::Application;
use iced::{window, Error, Settings};
use std::path::PathBuf;

fn main() -> Result<(), Error> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        println!("Invalid number of arguments. Correct usage:\n./server path_to_task_dir");
        std::process::exit(1);
    } else {
        println!(
            "Starting task \"{}\" with Server-v{}...\n",
            args[1], VERSION
        );
    }

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
        flags: path,
        // default_font: None,
        // default_text_size: 0,
        // text_multithreading: false,
        // antialiasing: false,
        exit_on_close_request: false,
        // try_opengles_first: false,
        ..Default::default()
    })
}
