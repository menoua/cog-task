use crate::backend::{ffmpeg, gst, MediaMode, MediaStream};
use crate::config::{Config, MediaBackend};
use crate::error;
use crate::error::Error::{BackendError, ResourceLoadError};
use crate::resource::FrameBuffer;
use eframe::egui::mutex::RwLock;
use eframe::egui::{TextureId, Vec2};
use eframe::epaint::TextureManager;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn stream_from_file(
    tex_manager: Arc<RwLock<TextureManager>>,
    path: &Path,
    config: &Config,
) -> Result<Stream, error::Error> {
    Stream::new(tex_manager, path, config)
}

#[derive(Clone)]
pub enum Stream {
    Gst(gst::Stream),
    Ffmpeg(ffmpeg::Stream),
}

impl Debug for Stream {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[video::Handle]")
    }
}

impl Stream {
    /// Create a new video object from a given video file.
    pub fn new(
        tex_manager: Arc<RwLock<TextureManager>>,
        path: &Path,
        config: &Config,
    ) -> Result<Self, error::Error> {
        {
            File::open(path).map_err(|e| {
                ResourceLoadError(format!("Failed to load video file ({path:?}):\n{e:#?}"))
            })?;
        }

        let media_backend = config.media_backend();
        match media_backend {
            MediaBackend::None => Err(BackendError(
                "Action type `stream` cannot be used with media backend `None`".to_owned(),
            )),
            MediaBackend::Ffmpeg => {
                ffmpeg::Stream::new(tex_manager, path, config).map(Stream::Ffmpeg)
            }
            MediaBackend::Gst => gst::Stream::new(tex_manager, path, config).map(Stream::Gst),
        }
    }

    /// Check if stream has reached its end.
    #[inline]
    pub fn eos(&self) -> bool {
        match self {
            Stream::Gst(stream) => stream.eos(),
            Stream::Ffmpeg(stream) => stream.eos(),
        }
    }

    /// Get the size/resolution of the video as `[width, height]`.
    #[inline]
    pub fn size(&self) -> [u32; 2] {
        match self {
            Stream::Gst(stream) => stream.size(),
            Stream::Ffmpeg(stream) => stream.size(),
        }
    }

    /// Get the framerate of the video as frames per second.
    #[inline]
    pub fn framerate(&self) -> f64 {
        match self {
            Stream::Gst(stream) => stream.framerate(),
            Stream::Ffmpeg(stream) => stream.framerate(),
        }
    }

    /// Get the number of audio channels.
    #[inline]
    pub fn channels(&self) -> u16 {
        match self {
            Stream::Gst(stream) => stream.channels(),
            Stream::Ffmpeg(stream) => stream.channels(),
        }
    }

    /// Get the media duration.
    #[inline]
    pub fn duration(&self) -> Duration {
        match self {
            Stream::Gst(stream) => stream.duration(),
            Stream::Ffmpeg(stream) => stream.duration(),
        }
    }

    /// Check if stream has a video channel.
    #[inline]
    pub fn has_video(&self) -> bool {
        match self {
            Stream::Gst(stream) => stream.has_video(),
            Stream::Ffmpeg(stream) => stream.has_video(),
        }
    }

    /// Check if stream has an audio channel.
    #[inline]
    pub fn has_audio(&self) -> bool {
        match self {
            Stream::Gst(stream) => stream.has_audio(),
            Stream::Ffmpeg(stream) => stream.has_audio(),
        }
    }

    // /// Set the volume multiplier of the audio.
    // /// `0.0` = 0% volume, `1.0` = 100% volume.
    // pub fn set_volume(&mut self, volume: f64) {
    //     self.source.set_property("volume", &volume);
    // }

    // /// Set if the audio is muted or not, without changing the volume.
    // pub fn set_muted(&mut self, muted: bool) {
    //     self.source.set_property("mute", &muted);
    // }

    // /// Get if the stream ended or not.
    // #[inline]
    // pub fn eos(&self) -> bool {
    //     self.is_eos
    // }

    // /// Get if the stream is paused.
    // #[inline]
    // pub fn paused(&self) -> bool {
    //     self.paused
    // }

    // /// Set if the media is paused or not.
    // pub fn set_paused(&mut self, paused: bool) -> Result<(), error::Error> {
    //     self.source
    //         .set_state(if paused {
    //             gst::State::Paused
    //         } else {
    //             gst::State::Playing
    //         })
    //         .map_err(|e| VideoDecodingError(format!("Failed to change video state:\n{e:#?}")))?;
    //     self.paused = paused;
    //     Ok(())
    // }

    /// Starts a stream; assumes it is at first frame and unpauses.
    pub fn start(&mut self) -> Result<(), error::Error> {
        match self {
            Stream::Gst(stream) => stream.start(),
            Stream::Ffmpeg(stream) => stream.start(),
        }
    }

    /// Restarts a stream; seeks to the first frame and unpauses, sets the `eos` flag to false.
    pub fn restart(&mut self) -> Result<(), error::Error> {
        match self {
            Stream::Gst(stream) => stream.restart(),
            Stream::Ffmpeg(stream) => stream.restart(),
        }
    }

    /// Pauses a stream
    pub fn pause(&mut self) -> Result<(), error::Error> {
        match self {
            Stream::Gst(stream) => stream.pause(),
            Stream::Ffmpeg(stream) => stream.pause(),
        }
    }

    pub fn process_bus(&mut self, looping: bool) -> Result<bool, error::Error> {
        match self {
            Stream::Gst(stream) => stream.process_bus(looping),
            Stream::Ffmpeg(stream) => stream.process_bus(looping),
        }
    }

    pub fn cloned(
        &self,
        frame: Arc<Mutex<Option<(TextureId, Vec2)>>>,
        media_mode: MediaMode,
        volume: Option<f32>,
    ) -> Result<Self, error::Error> {
        match self {
            Stream::Gst(stream) => stream.cloned(frame, media_mode, volume).map(Stream::Gst),
            Stream::Ffmpeg(stream) => stream.cloned(frame, media_mode, volume).map(Stream::Ffmpeg),
        }
    }

    pub fn pull_samples(&self) -> Result<(FrameBuffer, f64), error::Error> {
        match self {
            Stream::Gst(stream) => stream.pull_samples(),
            Stream::Ffmpeg(stream) => stream.pull_samples(),
        }
    }
}
