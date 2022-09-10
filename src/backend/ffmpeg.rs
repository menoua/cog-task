use crate::backend::{MediaMode, MediaStream};
use crate::config::Config;
use crate::error::Error;
use crate::resource::FrameBuffer;
use iced_native::image::Handle;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct Stream {
    _path: PathBuf,
    _frame_size: [u32; 2],
    _frame_rate: f64,
    _audio_chan: u16,
    _audio_rate: u32,
    _duration: Duration,
    _is_eos: bool,
    _paused: bool,
}

impl MediaStream for Stream {
    fn new(_path: &Path, _config: &Config, _media_mode: MediaMode) -> Result<Self, Error> {
        todo!()
    }

    fn cloned(&self, _volume: Option<f32>) -> Result<Self, Error> {
        todo!()
    }

    fn eos(&self) -> bool {
        todo!()
    }

    fn size(&self) -> [u32; 2] {
        todo!()
    }

    fn framerate(&self) -> f64 {
        todo!()
    }

    fn channels(&self) -> u16 {
        todo!()
    }

    fn duration(&self) -> Duration {
        todo!()
    }

    fn has_video(&self) -> bool {
        todo!()
    }

    fn has_audio(&self) -> bool {
        todo!()
    }

    fn start(&mut self) -> Result<(), Error> {
        todo!()
    }

    fn restart(&mut self) -> Result<(), Error> {
        todo!()
    }

    fn pause(&mut self) -> Result<(), Error> {
        todo!()
    }

    fn pull_samples(&self) -> Result<(FrameBuffer, f64), Error> {
        todo!()
    }

    fn process_bus(&mut self, _looping: bool) -> Result<bool, Error> {
        todo!()
    }

    fn set_callbacks(&mut self, _frame: Arc<Mutex<Option<Handle>>>) -> Result<(), Error> {
        todo!()
    }
}
