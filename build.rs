use itertools::Itertools;
use std::ffi::OsStr;
use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=src/action/include.rs");
    println!("cargo:rerun-if-changed=src/action/include/");

    let actions = fs::read_dir("src/action/include/")
        .unwrap()
        .into_iter()
        .map(|p| p.unwrap().path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().unwrap_or(OsStr::new("")).to_str() == Some("rs"))
        .map(|p| {
            p.with_extension("")
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        })
        .sorted()
        .map(|n| format!("    {n},"))
        .collect::<Vec<_>>()
        .join("\n");

    let content = format!("include_actions!(\n{}\n);\n", actions);

    let path = "src/action/include.rs";
    match fs::read_to_string(path) {
        Ok(current) if current == content => {}
        _ => {
            fs::write(path, content)
                .expect("Failed to generate src/action/include.rs automatically!");
        }
    }
}
