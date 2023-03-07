use eframe::egui::{ColorImage, Vec2};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Mask2D {
    alpha: Arc<Vec<f32>>,
    size: [usize; 2],
    scale: f32,
}

impl From<(ColorImage, Vec2)> for Mask2D {
    fn from(value: (ColorImage, Vec2)) -> Self {
        let (image, _) = value;
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
