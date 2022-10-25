#[cfg(not(any(feature = "gstreamer", feature = "ffmpeg")))]
compile_error!(
    "To enable streaming at least one backend should be enabled (\"gstreamer\" or \"ffmpeg\")."
);

#[cfg(feature = "ffmpeg")]
use crate::resource::ffmpeg;
#[cfg(feature = "gstreamer")]
use crate::resource::gst;
use crate::resource::{FrameBuffer, MediaMode, MediaStream};
use crate::server::{config::MediaBackend, Config};
use eframe::egui::mutex::RwLock;
use eframe::egui::{TextureId, Vec2};
use eframe::epaint::TextureManager;
use eyre::{eyre, Context, Result};
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn stream_from_file(
    tex_manager: Arc<RwLock<TextureManager>>,
    path: &Path,
    config: &Config,
) -> Result<Stream> {
    Stream::new(tex_manager, path, config)
}

#[derive(Clone)]
pub enum Stream {
    #[cfg(feature = "gstreamer")]
    Gst(gst::Stream),
    #[cfg(feature = "ffmpeg")]
    Ffmpeg(ffmpeg::Stream),
}

impl Debug for Stream {
    #[inline(always)]
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
    ) -> Result<Self> {
        {
            File::open(path).wrap_err_with(|| format!("Failed to load video file ({path:?})."))?;
        }

        let media_backend = config.media_backend();
        match media_backend {
            MediaBackend::None => Err(eyre!(
                "`Stream` action cannot be used without a media backend."
            )),
            #[cfg(feature = "ffmpeg")]
            MediaBackend::Ffmpeg => {
                ffmpeg::Stream::new(tex_manager, path, config).map(Stream::Ffmpeg)
            }
            #[cfg(feature = "gstreamer")]
            MediaBackend::Gst => gst::Stream::new(tex_manager, path, config).map(Stream::Gst),
        }
    }

    /// Check if stream has reached its end.
    #[inline(always)]
    pub fn eos(&self) -> bool {
        match self {
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
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.start(),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.start(),
        }
    }

    /// Restarts a stream; seeks to the first frame and unpauses, sets the `eos` flag to false.
    pub fn restart(&mut self) -> Result<()> {
        match self {
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.restart(),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.restart(),
        }
    }

    /// Pauses a stream
    pub fn pause(&mut self) -> Result<()> {
        match self {
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.pause(),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.pause(),
        }
    }

    pub fn process_bus(&mut self, looping: bool) -> Result<bool> {
        match self {
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.process_bus(looping),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.process_bus(looping),
        }
    }

    pub fn cloned(
        &self,
        frame: Arc<Mutex<Option<(TextureId, Vec2)>>>,
        media_mode: MediaMode,
        volume: f32,
    ) -> Result<Self> {
        match self {
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.cloned(frame, media_mode, volume).map(Stream::Gst),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.cloned(frame, media_mode, volume).map(Stream::Ffmpeg),
        }
    }

    pub fn pull_samples(&self) -> Result<(FrameBuffer, f64)> {
        match self {
            #[cfg(feature = "gstreamer")]
            Stream::Gst(stream) => stream.pull_samples(),
            #[cfg(feature = "ffmpeg")]
            Stream::Ffmpeg(stream) => stream.pull_samples(),
        }
    }
}
