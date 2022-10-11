use crate::config::Config;
use crate::error;
use crate::resource::stream::Stream;
use crate::resource::FrameBuffer;
use eframe::egui::mutex::RwLock;
use eframe::epaint::TextureManager;
use std::path::Path;
use std::sync::Arc;

pub fn video_from_file(
    tex_manager: Arc<RwLock<TextureManager>>,
    path: &Path,
    config: &Config,
) -> Result<(FrameBuffer, f64), error::Error> {
    Stream::new(tex_manager, path, config)?.pull_samples()
}
