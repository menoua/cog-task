use crate::error;
use crate::error::Error::ResourceLoadError;
use iced::pure::widget::{image, svg};
use std::path::Path;
// use tiny_skia::Pixmap;

pub fn image_from_file(path: &Path) -> Result<image::Handle, error::Error> {
    let bytes = std::fs::read(&path).map_err(|e| {
        ResourceLoadError(format!("Failed to read image file: `{path:?}`:\n{e:#?}"))
    })?;
    Ok(image_from_bytes(bytes, path))
}

#[inline(always)]
pub fn image_from_bytes(bytes: Vec<u8>, _path: &Path) -> image::Handle {
    image::Handle::from_memory(bytes)
}

pub fn svg_from_file(path: &Path) -> Result<svg::Handle, error::Error> {
    let bytes = std::fs::read(&path).map_err(|e| {
        ResourceLoadError(format!("Failed to read image file: `{path:?}`:\n{e:#?}"))
    })?;
    Ok(svg_from_bytes(bytes, path))
}

#[inline(always)]
pub fn svg_from_bytes(bytes: Vec<u8>, _path: &Path) -> svg::Handle {
    svg::Handle::from_memory(bytes)
}
