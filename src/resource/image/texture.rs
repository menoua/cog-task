use crate::resource::{image_from_file, svg_from_file};
use eframe::egui::mutex::RwLock;
use eframe::egui::{ColorImage, ImageData, TextureId, TextureOptions, Vec2};
use eframe::epaint;
use eyre::Result;
use std::path::Path;
use std::sync::Arc;

pub type Texture = (TextureId, Vec2);
pub type ArcTexManager = Arc<RwLock<epaint::TextureManager>>;

pub fn texture_from_image(
    tex_manager: ArcTexManager,
    name: &str,
    image: ColorImage,
    size: Vec2,
) -> Texture {
    let texture = tex_manager.write().alloc(
        name.to_owned(),
        ImageData::Color(image),
        TextureOptions::NEAREST,
    );

    (texture, size)
}

pub fn texture_from_image_file(tex_manager: ArcTexManager, path: &Path) -> Result<Texture> {
    let name = path.to_str().unwrap().to_owned();
    let (image, size) = image_from_file(path)?;
    let texture = tex_manager
        .write()
        .alloc(name, ImageData::Color(image), TextureOptions::NEAREST);

    Ok((texture, size))
}

pub fn texture_from_svg_file(tex_manager: ArcTexManager, path: &Path) -> Result<Texture> {
    let name = path.to_str().unwrap().to_owned();
    let (image, size) = svg_from_file(path)?;
    let texture = tex_manager
        .write()
        .alloc(name, ImageData::Color(image), TextureOptions::NEAREST);

    Ok((texture, size))
}
