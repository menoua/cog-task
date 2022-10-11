use crate::config::Config;
use crate::error;
use crate::resource::FrameBuffer;
use eframe::egui::mutex::RwLock;
use eframe::egui::{TextureId, Vec2};
use eframe::epaint::TextureManager;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub mod ffmpeg;
pub mod gst;

pub trait MediaStream
where
    Self: Sized,
{
    fn new(
        tex_manager: Arc<RwLock<TextureManager>>,
        path: &Path,
        config: &Config,
        media_mode: MediaMode,
    ) -> Result<Self, error::Error>;
    fn cloned(
        &self,
        frame: Arc<Mutex<Option<(TextureId, Vec2)>>>,
        volume: Option<f32>,
    ) -> Result<Self, error::Error>;

    fn eos(&self) -> bool;
    fn size(&self) -> [u32; 2];
    fn framerate(&self) -> f64;
    fn channels(&self) -> u16;
    fn duration(&self) -> Duration;
    fn has_video(&self) -> bool {
        self.size().iter().sum::<u32>() > 0
    }
    fn has_audio(&self) -> bool {
        self.channels() > 0
    }

    fn start(&mut self) -> Result<(), error::Error>;
    fn restart(&mut self) -> Result<(), error::Error>;
    fn pause(&mut self) -> Result<(), error::Error>;
    fn pull_samples(&self) -> Result<(FrameBuffer, f64), error::Error>;
    fn process_bus(&mut self, looping: bool) -> Result<bool, error::Error>;
}

#[derive(Debug, Clone)]
pub enum MediaMode {
    Query,
    Normal,
    Muted,
    SansIntTrigger,
    WithExtTrigger(PathBuf),
}
