use eframe::egui::{ColorImage, Vec2};
use eyre::{eyre, Context, Result};
use std::fs;
use std::path::Path;

pub mod mask;
pub mod texture;

pub use mask::Mask2D;
pub use texture::Texture;

pub fn image_from_file(path: &Path) -> Result<(ColorImage, Vec2)> {
    let bytes = fs::read(path).wrap_err_with(|| format!("Failed to read image file: {path:?}"))?;
    image_from_bytes(&bytes).wrap_err_with(|| format!("Failed to decode image file: {path:?}"))
}

pub fn image_from_bytes(bytes: &[u8]) -> Result<(ColorImage, Vec2)> {
    let image = image::load_from_memory(bytes)
        .wrap_err_with(|| "Failed to decode image from bytes.".to_owned())?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    let image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
    Ok((image, Vec2::new(size[0] as _, size[1] as _)))
}

pub fn svg_from_file(path: &Path) -> Result<(ColorImage, Vec2)> {
    let bytes = fs::read(path).wrap_err_with(|| format!("Failed to read image file: {path:?}"))?;
    svg_from_bytes(&bytes).wrap_err_with(|| format!("Failed to decode image file: {path:?}"))
}

pub fn svg_from_bytes(bytes: &[u8]) -> Result<(ColorImage, Vec2)> {
    let opt = usvg::Options::default();
    // opt.fontdb.load_system_fonts();

    let rtree = usvg::Tree::from_data(bytes, &opt)
        .wrap_err_with(|| "Failed to decode SVG from bytes.".to_owned())?;
    let orig_size = rtree.size;

    let [width, height] = [1920, 1080];
    let scale = (width as f64 / orig_size.width()).min(height as f64 / orig_size.height());
    let [width, height] = [
        (scale * orig_size.width()).round() as _,
        (scale * orig_size.height()).round() as _,
    ];

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| eyre!("Failed to create SVG Pixmap of size {width}x{height}."))?;

    resvg::render(
        &rtree,
        usvg::FitTo::Size(width, height),
        Default::default(),
        pixmap.as_mut(),
    )
    .ok_or_else(|| eyre!("Failed to decode SVG from bytes."))?;

    let image = ColorImage::from_rgba_unmultiplied([width as _, height as _], pixmap.data());

    Ok((
        image,
        Vec2::new(orig_size.width() as _, orig_size.height() as _),
    ))
}
