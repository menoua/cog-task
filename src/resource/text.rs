use regex;
use regex::Regex;
use std::path::PathBuf;
use std::str::FromStr;

pub fn text_or_file(text: &str) -> Option<PathBuf> {
    let re = Regex::new(r"^!!\[(.*)]$").unwrap();
    if let Some(m) = re.captures(text) {
        if let Ok(path) = PathBuf::from_str(&m[1]) {
            return Some(path);
        }
    }

    None
}
