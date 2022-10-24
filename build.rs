use heck::ToUpperCamelCase;
use itertools::Itertools;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/action/include.rs");
    println!("cargo:rerun-if-changed=src/action/core/");
    println!("cargo:rerun-if-changed=src/action/core/mod.rs");
    println!("cargo:rerun-if-changed=src/action/extra/");
    println!("cargo:rerun-if-changed=src/action/extra/mod.rs");

    let core = fs::read_dir("src/action/core/")
        .unwrap()
        .into_iter()
        .map(|p| p.unwrap().path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().unwrap_or_else(|| OsStr::new("")).to_str() == Some("rs"))
        .map(|p| {
            p.with_extension("")
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        })
        .filter(|n| n != "mod")
        .sorted()
        .collect_vec();

    let core_stateful = core
        .clone()
        .into_iter()
        .filter(|n| {
            fs::read_to_string(
                Path::new("src/action/core/").join(Path::new(n).with_extension("rs")),
            )
            .unwrap()
            .contains(&format!("Stateful{}", n.to_upper_camel_case()))
        })
        .collect_vec();

    let extra = fs::read_dir("src/action/extra/")
        .unwrap()
        .into_iter()
        .map(|p| p.unwrap().path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().unwrap_or_else(|| OsStr::new("")).to_str() == Some("rs"))
        .map(|p| {
            p.with_extension("")
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        })
        .filter(|n| n != "mod")
        .sorted()
        .collect_vec();

    let extra_stateful = extra
        .clone()
        .into_iter()
        .filter(|n| {
            fs::read_to_string(
                Path::new("src/action/extra/").join(Path::new(n).with_extension("rs")),
            )
            .unwrap()
            .contains(&format!("Stateful{}", n.to_upper_camel_case()))
        })
        .collect_vec();

    let mut features: HashMap<String, Vec<String>> = HashMap::new();

    for action in core.iter() {
        let line = fs::read_to_string(
            Path::new("src/action/core/").join(Path::new(action).with_extension("rs")),
        )
        .unwrap()
        .lines()
        .next()
        .unwrap_or("")
        .to_owned();

        if line.starts_with("//!") {
            line.strip_prefix("//!")
                .unwrap()
                .trim()
                .split(" *, *")
                .for_each(|f| {
                    features
                        .entry(action.clone())
                        .or_default()
                        .push(f.to_owned());
                });
        }
    }

    for action in extra.iter() {
        let line = fs::read_to_string(
            Path::new("src/action/extra/").join(Path::new(action).with_extension("rs")),
        )
        .unwrap()
        .lines()
        .next()
        .unwrap_or("")
        .to_owned();

        if line.starts_with("//!") {
            line.strip_prefix("//!")
                .unwrap()
                .trim()
                .split(" *, *")
                .for_each(|f| {
                    features
                        .entry(action.clone())
                        .or_default()
                        .push(f.to_owned());
                });
        }
    }

    for (_, v) in features.iter_mut() {
        v.sort();
    }

    let content = format!(
        "\
        // This file is automatically generated by the crate build script.\n\
        // DO NOT MODIFY THIS FILE MANUALLY! CHANGES WILL BE REVERTED.\n\
        {}\
        {}\
        ",
        if core.is_empty() { "" } else { "\n" },
        core.iter()
            .map(|n| {
                if let Some(f) = features.get(n) {
                    format!("#[cfg(feature = \"{}\")]\npub mod {n};\n", f.join(", "))
                } else {
                    format!("pub mod {n};\n")
                }
            })
            .collect::<Vec<_>>()
            .join(""),
    );

    let path = "src/action/core/mod.rs";
    match fs::read_to_string(path) {
        Ok(current) if current == content => {}
        _ => {
            fs::write(path, content)
                .expect("Failed to generate src/action/core/mod.rs automatically!");
        }
    }

    let content = format!(
        "\
        // This file is automatically generated by the crate build script.\n\
        // DO NOT MODIFY THIS FILE MANUALLY! CHANGES WILL BE REVERTED.\n\
        {}\
        {}\
        ",
        if extra.is_empty() { "" } else { "\n" },
        extra
            .iter()
            .map(|n| if let Some(f) = features.get(n) {
                format!("#[cfg(feature = \"{}\")]\npub mod {n};\n", f.join(", "))
            } else {
                format!("pub mod {n};\n")
            })
            .collect::<Vec<_>>()
            .join(""),
    );

    let path = "src/action/extra/mod.rs";
    match fs::read_to_string(path) {
        Ok(current) if current == content => {}
        _ => {
            fs::write(path, content)
                .expect("Failed to generate src/action/extra/mod.rs automatically!");
        }
    }

    let has_actions = !core.is_empty();
    let has_stateful = !core.is_empty();

    let content = format!(
        "\
        // This file is automatically generated by the crate build script.\n\
        // DO NOT MODIFY THIS FILE MANUALLY! CHANGES WILL BE REVERTED.\n\n\
        include_actions!({}{}{});\n\n\
        include_stateful_actions!({}{}{});\n\
        ",
        core.into_iter()
            .map(|n| {
                if let Some(f) = features.get(&n) {
                    format!(
                        "\n    core::{n}@({}),",
                        f.iter().map(|f| format!("\"{f}\"")).join(", ")
                    )
                } else {
                    format!("\n    core::{n}@(),")
                }
            })
            .collect::<Vec<_>>()
            .join(""),
        extra
            .into_iter()
            .map(|n| {
                if let Some(f) = features.get(&n) {
                    format!(
                        "\n    extra::{n}@({}),",
                        f.iter().map(|f| format!("\"{f}\"")).join(", ")
                    )
                } else {
                    format!("\n    extra::{n}@(),")
                }
            })
            .collect::<Vec<_>>()
            .join(""),
        if has_actions {
            "\n".to_owned()
        } else {
            "".to_owned()
        },
        core_stateful
            .into_iter()
            .map(|n| {
                if let Some(f) = features.get(&n) {
                    format!(
                        "\n    core::{n}@({}),",
                        f.iter().map(|f| format!("\"{f}\"")).join(", ")
                    )
                } else {
                    format!("\n    core::{n}@(),")
                }
            })
            .collect::<Vec<_>>()
            .join(""),
        extra_stateful
            .into_iter()
            .map(|n| {
                if let Some(f) = features.get(&n) {
                    format!(
                        "\n    extra::{n}@({}),",
                        f.iter().map(|f| format!("\"{f}\"")).join(", ")
                    )
                } else {
                    format!("\n    extra::{n}@(),")
                }
            })
            .collect::<Vec<_>>()
            .join(""),
        if has_stateful {
            "\n".to_owned()
        } else {
            "".to_owned()
        }
    );

    let path = "src/action/include.rs";
    match fs::read_to_string(path) {
        Ok(current) if current == content => {}
        _ => {
            fs::write(path, content)
                .expect("Failed to generate src/action/include.rs automatically!");
        }
    }
}
