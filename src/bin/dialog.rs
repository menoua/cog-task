use native_dialog::{FileDialog, MessageDialog, MessageType};

fn main() {
    let path = FileDialog::new()
        .set_location(std::env::current_dir().unwrap().to_str().unwrap())
        .show_open_single_dir()
        .unwrap();

    let path = match path {
        Some(path) => path,
        None => return,
    };

    let yes = MessageDialog::new()
        .set_type(MessageType::Info)
        .set_title("Do you want to open the file?")
        .set_text(&format!("{:#?}", path))
        .show_confirm()
        .unwrap();

    if yes {
        println!("Haha");
    }
}
