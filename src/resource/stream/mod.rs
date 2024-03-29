use crate::resource::AudioChannel;
use crate::server::Config;
use eframe::egui::mutex::RwLock;
use eframe::egui::{TextureId, Vec2};
use eframe::epaint::TextureManager;
use eyre::{eyre, Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(feature = "ffmpeg")]
mod ffmpeg;
#[cfg(feature = "gstreamer")]
mod gst;

pub type FrameBuffer = Arc<Vec<(TextureId, Vec2)>>;
pub type VideoBuffer = (FrameBuffer, f64);

#[derive(Clone)]
pub enum Stream {
    None,
    #[cfg(feature = "gstreamer")]
    Gst(gst::Stream),
    #[cfg(feature = "ffmpeg")]
    Ffmpeg(ffmpeg::Stream),
}

pub fn stream_from_file(
    tex_manager: Arc<RwLock<TextureManager>>,
    path: &Path,
    config: &Config,
) -> Result<Stream> {
    Stream::new(tex_manager, path, config)
}

pub fn video_from_file(
    tex_manager: Arc<RwLock<TextureManager>>,
    path: &Path,
    config: &Config,
) -> Result<(FrameBuffer, f64)> {
    Stream::new(tex_manager, path, config)?.pull_samples()
}

pub trait MediaStream
where
    Self: Sized,
{
    fn new(tex_manager: Arc<RwLock<TextureManager>>, path: &Path, config: &Config) -> Result<Self>;
    fn cloned(
        &self,
        frame: Arc<Mutex<Option<(TextureId, Vec2)>>>,
        media_mode: StreamMode,
        volume: f32,
    ) -> Result<Self>;

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

    fn start(&mut self) -> Result<()>;
    fn restart(&mut self) -> Result<()>;
    fn pause(&mut self) -> Result<()>;
    fn pull_samples(&self) -> Result<(FrameBuffer, f64)>;
    fn process_bus(&mut self, looping: bool) -> Result<bool>;
}

#[derive(Debug, Clone)]
pub enum StreamMode {
    Query,
    Normal(AudioChannel),
    Muted,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamBackend {
    None,
    Inherit,
    #[cfg(feature = "gstreamer")]
    Gst,
    #[cfg(feature = "ffmpeg")]
    Ffmpeg,
}

impl Default for StreamBackend {
    #[inline(always)]
    fn default() -> Self {
        StreamBackend::Inherit
    }
}

impl StreamBackend {
    pub fn or(&self, other: &Self) -> Self {
        if let Self::Inherit = self {
            *other
        } else {
            *self
        }
    }
}

impl Debug for Stream {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[video::Handle]")
    }
}

impl Stream {
    /// Create a new video object from a given video file.
    #[allow(unused_variables)]
    pub fn new(
        tex_manager: Arc<RwLock<TextureManager>>,
        path: &Path,
        config: &Config,
    ) -> Result<Self> {
        {
            File::open(path).wrap_err_with(|| format!("Failed to open stream file ({path:?})."))?;
        }

        let media_backend = config.stream_backend();
        match media_backend {
            StreamBackend::None => Err(eyre!("Cannot init a stream with backend=None.")),
            StreamBackend::Inherit => Err(eyre!("Cannot init a stream with backend=Inherit.")),
            #[cfg(feature = "ffmpeg")]
            StreamBackend::Ffmpeg => {
                ffmpeg::Stream::new(tex_manager, path, config).map(Stream::Ffmpeg)
            }
            #[cfg(feature = "gstreamer")]
            StreamBackend::Gst => gst::Stream::new(tex_manager, path, config).map(Stream::Gst),
        }
    }

    /// Check if stream has reached its end.
    #[inline(always)]
    pub fn eos(&self) -> bool {
        match self {
            Stream::None => true,
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.eos(),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.eos(),
        }
    }

    /// Get the size/resolution of the video as `[width, height]`.
    #[inline(always)]
    pub fn size(&self) -> [u32; 2] {
        match self {
            Stream::None => [0, 0],
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.size(),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.size(),
        }
    }

    /// Get the framerate of the video as frames per second.
    #[inline(always)]
    pub fn framerate(&self) -> f64 {
        match self {
            Stream::None => 0.0,
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.framerate(),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.framerate(),
        }
    }

    /// Get the number of audio channels.
    #[inline(always)]
    pub fn channels(&self) -> u16 {
        match self {
            Stream::None => 0,
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.channels(),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.channels(),
        }
    }

    /// Get the media duration.
    #[inline(always)]
    pub fn duration(&self) -> Duration {
        match self {
            Stream::None => Duration::default(),
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.duration(),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.duration(),
        }
    }

    /// Check if stream has a video channel.
    #[inline(always)]
    pub fn has_video(&self) -> bool {
        match self {
            Stream::None => false,
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.has_video(),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.has_video(),
        }
    }

    /// Check if stream has an audio channel.
    #[inline(always)]
    pub fn has_audio(&self) -> bool {
        match self {
            Stream::None => false,
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.has_audio(),
            #[cfg(feature = "ffmpeg")]
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
    pub fn start(&mut self) -> Result<()> {
        match self {
            Stream::None => Err(eyre!("Cannot start stream with backend=None.")),
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.start(),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.start(),
        }
    }

    /// Restarts a stream; seeks to the first frame and unpauses, sets the `eos` flag to false.
    pub fn restart(&mut self) -> Result<()> {
        match self {
            Stream::None => Err(eyre!("Cannot restart stream with backend=None.")),
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.restart(),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.restart(),
        }
    }

    /// Pauses a stream
    pub fn pause(&mut self) -> Result<()> {
        match self {
            Stream::None => Err(eyre!("Cannot pause stream with backend=None.")),
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.pause(),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.pause(),
        }
    }

    #[allow(unused_variables)]
    pub fn process_bus(&mut self, looping: bool) -> Result<bool> {
        match self {
            Stream::None => Err(eyre!("Cannot process bus for stream with backend=None.")),
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.process_bus(looping),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.process_bus(looping),
        }
    }

    #[allow(unused_variables)]
    pub fn cloned(
        &self,
        frame: Arc<Mutex<Option<(TextureId, Vec2)>>>,
        mode: StreamMode,
        volume: f32,
    ) -> Result<Self> {
        match self {
            Stream::None => Err(eyre!("Cloning stream with backend=None is pointless.")),
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.cloned(frame, mode, volume).map(Stream::Gst),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.cloned(frame, mode, volume).map(Stream::Ffmpeg),
        }
    }

    pub fn pull_samples(&self) -> Result<(FrameBuffer, f64)> {
        match self {
            Stream::None => Err(eyre!("Cannot pull samples from stream with backend=None.")),
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.pull_samples(),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.pull_samples(),
        }
    }
}
