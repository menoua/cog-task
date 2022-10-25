use eframe::egui::mutex::RwLock;
use eframe::egui::{ColorImage, ImageData, TextureFilter, Vec2};
use eframe::{egui, epaint};
use egui::TextureId;
use eyre::{eyre, Context, Result};
use std::fs;
use std::path::Path;
use std::sync::Arc;

pub fn image_from_file(
    tex_manager: Arc<RwLock<epaint::TextureManager>>,
    path: &Path,
) -> Result<(TextureId, Vec2)> {
    let bytes = fs::read(&path).wrap_err_with(|| format!("Failed to read image file: {path:?}"))?;
    image_from_bytes(tex_manager, &bytes, path)
}

#[inline]
pub fn image_from_bytes(
    tex_manager: Arc<RwLock<epaint::TextureManager>>,
    bytes: &[u8],
    path: &Path,
) -> Result<(TextureId, Vec2)> {
    let image = image::load_from_memory(bytes)
        .wrap_err_with(|| format!("Failed to decode image: {path:?}"))?;
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
) -> Result<(TextureId, Vec2)> {
    let bytes = fs::read(&path).wrap_err_with(|| format!("Failed to read image file: {path:?}"))?;
    svg_from_bytes(tex_manager, &bytes, path)
}

#[inline]
pub fn svg_from_bytes(
    tex_manager: Arc<RwLock<epaint::TextureManager>>,
    bytes: &[u8],
    path: &Path,
) -> Result<(TextureId, Vec2)> {
    let mut opt = usvg::Options::default();
    opt.fontdb.load_system_fonts();

    let rtree = usvg::Tree::from_data(bytes, &opt.to_ref())
        .wrap_err_with(|| format!("Failed to decode SVG: {path:?}"))?;
    let orig_size = rtree.size;

    let [width, height] = [1920, 1080];
    let scale = (width as f64 / orig_size.width()).min(height as f64 / orig_size.height());
    let [width, height] = [
        (scale * orig_size.width() as f64).round() as _,
        (scale * orig_size.height() as f64).round() as _,
    ];

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| eyre!("Failed to create SVG Pixmap of size {width}x{height}: {path:?}"))?;

    resvg::render(
        &rtree,
        usvg::FitTo::Size(width, height),
        Default::default(),
        pixmap.as_mut(),
    )
    .ok_or_else(|| eyre!("Failed to decode SVG: {path:?}"))?;

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
