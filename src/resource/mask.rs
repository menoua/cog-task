use eframe::egui::{ColorImage, Vec2};
use eyre::{eyre, Context, Result};
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Mask2D {
    alpha: Arc<Vec<f32>>,
    size: [usize; 2],
    scale: f32,
}

pub fn mask_from_image_file(path: &Path) -> Result<Mask2D> {
    let bytes = fs::read(path).wrap_err_with(|| format!("Failed to read image file: {path:?}"))?;
    mask_from_image_bytes(&bytes, path)
}

#[inline]
pub fn mask_from_image_bytes(bytes: &[u8], path: &Path) -> Result<Mask2D> {
    let image = image::load_from_memory(bytes)
        .wrap_err_with(|| format!("Failed to decode image: {path:?}"))?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    let image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
    Ok(Mask2D::from(image))
}

pub fn mask_from_svg_file(path: &Path) -> Result<Mask2D> {
    let bytes = fs::read(path).wrap_err_with(|| format!("Failed to read image file: {path:?}"))?;
    mask_from_svg_bytes(&bytes, path)
}

#[inline]
pub fn mask_from_svg_bytes(bytes: &[u8], path: &Path) -> Result<Mask2D> {
    let opt = usvg::Options::default();
    // opt.fontdb.load_system_fonts();

    let rtree = usvg::Tree::from_data(bytes, &opt)
        .wrap_err_with(|| format!("Failed to decode SVG: {path:?}"))?;
    let orig_size = rtree.size;

    let [width, height] = [1920, 1080];
    let scale = (width as f64 / orig_size.width()).min(height as f64 / orig_size.height());
    let [width, height] = [
        (scale * orig_size.width()).round() as _,
        (scale * orig_size.height()).round() as _,
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
    Ok(Mask2D::from(image))
}

impl From<ColorImage> for Mask2D {
    fn from(image: ColorImage) -> Self {
        let alpha: Vec<_> = image
            .pixels
            .into_iter()
            .map(|p| p.a() as f32 / u8::MAX as f32)
            .collect();

        Mask2D {
            alpha: Arc::new(alpha),
            size: image.size,
            scale: 1.0,
        }
    }
}

impl Mask2D {
    pub fn value_at(&self, loc: Vec2) -> f32 {
        let x = (loc.x * self.scale - 0.5).round().max(0.0) as usize;
        let y = (loc.y * self.scale - 0.5).round().max(0.0) as usize;

        if x > self.size[0] || y > self.size[1] {
            return 0.0;
        }

        let (x, y) = (x.min(self.size[0] - 1), y.min(self.size[1] - 1));
        *self.alpha.get(y * self.size[0] + x).unwrap_or(&0.0)
    }

    #[inline]
    pub fn contains(&self, loc: Vec2) -> bool {
        self.value_at(loc) > 0.0
    }

    #[inline]
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.size[0] as f32, self.size[1] as f32)
    }

    #[inline]
    pub fn scaled(&self, scale: f32) -> Self {
        Self {
            alpha: self.alpha.clone(),
            size: self.size,
            scale,
        }
    }
}
