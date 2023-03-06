use cog_task::assets::VERSION;
use cog_task::server::Server;
use eyre::{Context, Result};
use sha2::{Digest, Sha256};
use std::env::current_exe;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        println!("Invalid number of arguments. Correct usage:\n./server path_to_task_dir");
        std::process::exit(1);
    } else {
        println!("Starting task \"{}\" with Server-v{}...", args[1], VERSION);
    }

    let mut bin = current_exe().wrap_err("Could not obtain path to current executable.")?;
    while bin.is_symlink() {
        bin = bin
            .read_link()
            .wrap_err("Could not dereference symlink to current executable.")?;
    }
    let mut hasher = Sha256::default();
    hasher.update(&std::fs::read(bin).unwrap());
    let bin_hash = hex::encode(hasher.finalize());

    let path = PathBuf::from(&args[1]);

    Server::new(path, bin_hash)?.run()
}
