use crate::error;
use crate::error::Error::ResourceLoadError;
use eframe::egui::mutex::RwLock;
use eframe::egui::{ColorImage, ImageData, TextureFilter, Vec2};
use eframe::{egui, epaint};
use egui::TextureId;
use epaint::TextureManager;
use std::path::Path;
use std::sync::Arc;

pub fn image_from_file(
    tex_manager: Arc<RwLock<epaint::TextureManager>>,
    path: &Path,
) -> Result<(TextureId, Vec2), error::Error> {
    let bytes = std::fs::read(&path).map_err(|e| {
        ResourceLoadError(format!("Failed to read image file: `{path:?}`:\n{e:#?}"))
    })?;
    image_from_bytes(tex_manager, &bytes, path)
}

#[inline(always)]
pub fn image_from_bytes(
    tex_manager: Arc<RwLock<epaint::TextureManager>>,
    bytes: &[u8],
    path: &Path,
) -> Result<(TextureId, Vec2), error::Error> {
    let image = image::load_from_memory(bytes)
        .map_err(|e| ResourceLoadError(format!("Failed to decode image \"{path:?}\": {e:?}")))?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    let image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

    Ok((
        tex_manager.write().alloc(
            path.to_str().unwrap().to_owned(),
            ImageData::Color(image),
            TextureFilter::Nearest,
        ),
        Vec2::new(size[0] as _, size[1] as _),
    ))
}

pub fn svg_from_file(
    tex_manager: Arc<RwLock<epaint::TextureManager>>,
    path: &Path,
) -> Result<(TextureId, Vec2), error::Error> {
    let bytes = std::fs::read(&path).map_err(|e| {
        ResourceLoadError(format!("Failed to read image file: `{path:?}`:\n{e:#?}"))
    })?;
    svg_from_bytes(tex_manager, &bytes, path)
}

#[inline(always)]
pub fn svg_from_bytes(
    tex_manager: Arc<RwLock<epaint::TextureManager>>,
    bytes: &[u8],
    path: &Path,
) -> Result<(TextureId, Vec2), error::Error> {
    let mut opt = usvg::Options::default();
    opt.fontdb.load_system_fonts();

    let rtree = usvg::Tree::from_data(bytes, &opt.to_ref())
        .map_err(|e| ResourceLoadError(format!("Failed to decode SVG \"{path:?}\": {e:?}")))?;
    let orig_size = rtree.svg_node().size;

    let [width, height] = [1920, 1080];
    let scale = (width as f64 / orig_size.width()).min((height as f64 / orig_size.height()));
    let [width, height] = [
        (scale * orig_size.width() as f64).round() as _,
        (scale * orig_size.height() as f64).round() as _,
    ];

    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or_else(|| {
        ResourceLoadError(format!(
            "Failed to create SVG Pixmap of size {width}x{height} for \"{path:?}\""
        ))
    })?;

    resvg::render(
        &rtree,
        usvg::FitTo::Size(width, height),
        Default::default(),
        pixmap.as_mut(),
    )
    .ok_or_else(|| ResourceLoadError(format!("Failed to decode SVG \"{path:?}\"")))?;

    let image = ColorImage::from_rgba_unmultiplied([width as _, height as _], pixmap.data());

    Ok((
        tex_manager.write().alloc(
            path.to_str().unwrap().to_owned(),
            ImageData::Color(image),
            TextureFilter::Nearest,
        ),
        Vec2::new(orig_size.width() as _, orig_size.height() as _),
    ))
}
