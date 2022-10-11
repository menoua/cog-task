use crate::error;
use crate::error::Error::ResourceLoadError;
use eframe::egui::mutex::RwLock;
use eframe::egui::{ColorImage, ImageData, TextureFilter, Vec2};
use eframe::{egui, epaint};
use egui::TextureId;
use egui_extras::image::{load_image_bytes, load_svg_bytes};
use epaint::TextureManager;
use std::path::Path;
use std::sync::Arc;
// use tiny_skia::Pixmap;

pub fn image_from_file(
    tex_manager: Arc<RwLock<epaint::TextureManager>>,
    path: &Path,
) -> Result<(TextureId, Vec2), error::Error> {
    let bytes = std::fs::read(&path).map_err(|e| {
        ResourceLoadError(format!("Failed to read image file: `{path:?}`:\n{e:#?}"))
    })?;
    image_from_bytes(tex_manager, bytes, path)
}

#[inline(always)]
pub fn image_from_bytes(
    tex_manager: Arc<RwLock<epaint::TextureManager>>,
    bytes: Vec<u8>,
    path: &Path,
) -> Result<(TextureId, Vec2), error::Error> {
    let image = load_image_bytes(&bytes)
        .map_err(|e| ResourceLoadError(format!("Failed to decode image \"{path:?}\": {e}")))?;
    let size = Vec2::new(image.size[0] as _, image.size[1] as _);
    Ok((
        tex_manager.write().alloc(
            path.to_str().unwrap().to_owned(),
            ImageData::Color(image),
            TextureFilter::Nearest,
        ),
        size,
    ))
}

pub fn svg_from_file(
    tex_manager: Arc<RwLock<epaint::TextureManager>>,
    path: &Path,
) -> Result<(TextureId, Vec2), error::Error> {
    let bytes = std::fs::read(&path).map_err(|e| {
        ResourceLoadError(format!("Failed to read image file: `{path:?}`:\n{e:#?}"))
    })?;
    svg_from_bytes(tex_manager, bytes, path)
}

#[inline(always)]
pub fn svg_from_bytes(
    tex_manager: Arc<RwLock<epaint::TextureManager>>,
    bytes: Vec<u8>,
    path: &Path,
) -> Result<(TextureId, Vec2), error::Error> {
    let image = load_svg_bytes(&bytes)
        .map_err(|e| ResourceLoadError(format!("Failed to decode image \"{path:?}\": {e}")))?;
    let size = Vec2::new(image.size[0] as _, image.size[1] as _);
    Ok((
        tex_manager.write().alloc(
            path.to_str().unwrap().to_owned(),
            ImageData::Color(image),
            TextureFilter::Nearest,
        ),
        size,
    ))
}
