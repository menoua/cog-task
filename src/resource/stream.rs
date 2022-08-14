use crate::config::Config;
use crate::error;
use crate::error::Error::{InternalError, ResourceLoadError, VideoDecodingError};
use crate::resource::FrameBuffer;
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;
use iced::pure::widget::image;
use num_rational::Rational32;
use num_traits::ToPrimitive;
use once_cell::sync::OnceCell;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{env, thread};
use thiserror::Error;

static GST_INIT: OnceCell<()> = OnceCell::new();

pub fn gst_init() -> Result<(), error::Error> {
    if GST_INIT.get().is_some() {
        return Ok(());
    }

    let plugin_env = "GST_PLUGIN_PATH";
    if env::var(plugin_env).is_err() {
        let mut list = vec![];

        if let Ok(home) = env::var("HOME") {
            let path = format!("{home}/.gstreamer-0.10");
            if Path::new(&path).exists() {
                list.push(path);
            }
        }

        let path = "/usr/local/lib/gstreamer-1.0";
        if Path::new(path).exists() {
            list.push(path.to_owned());
        }

        env::set_var(plugin_env, list.join(":"));
    }

    gst::init()
        .map(|r| {
            GST_INIT.set(()).expect("Tried to init GStreamer twice");
            r
        })
        .map_err(|e| {
            VideoDecodingError(format!(
                "Failed to initialize GStreamer: required because there is a video element \
                in this block:\n{e:#?}",
            ))
        })
}

pub fn stream_from_file(path: &Path, _config: &Config) -> Result<Stream, error::Error> {
    Stream::new(path)
}

/// A video handle that uses GStreamer to stream video content.
/// This `struct` and its associated `impl` is a simplified version of the
/// `VideoPlayer` struct found at: https://github.com/jazzfool/iced_video_player.
pub struct Stream {
    uri: String,
    source: gst::Bin,
    bus: gst::Bus,
    app_sink: Option<gst_app::AppSink>,
    frame_size: [u32; 2],
    frame_rate: f64,
    audio_chan: u16,
    audio_rate: u32,
    duration: Duration,
    is_eos: bool,
    paused: bool,
}

impl Debug for Stream {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[video::Handle]")
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.source
            .set_state(gst::State::Null)
            .expect("Failed to drop video handle");
    }
}

impl Stream {
    /// Create a new video object from a given video file.
    pub fn new(path: &Path) -> Result<Self, error::Error> {
        gst_init()?;

        {
            File::open(path).map_err(|e| {
                ResourceLoadError(format!("Failed to load video file ({path:?}):\n{e:#?}"))
            })?;
        }

        let uri = format!(
            "file://{}",
            path.canonicalize()
                .map_err(|e| ResourceLoadError(format!(
                    "Failed to canonicalize video file path: {path:?}:\n{e:#?}"
                )))?
                .as_os_str()
                .to_str()
                .unwrap()
        );

        let source = gst_launch(&uri, true, None)?;
        let bus = source.bus().unwrap();

        let video_sink = video_sink_from_source(&source, false);
        let (width, height, frame_rate) = match video_sink.as_ref() {
            Some(sink) => video_meta_from_sink(sink)?,
            None => (0, 0, 0.0),
        };

        let audio_sink = audio_sink_from_source(&source, false);
        let (audio_chan, audio_rate) = match audio_sink.as_ref() {
            Some(sink) => audio_meta_from_sink(sink)?,
            None => (0, 0),
        };

        let duration = Duration::from_nanos(
            source
                .query_duration::<gst::ClockTime>()
                .ok_or(Error::Duration)
                .unwrap()
                .nseconds(),
        );

        source.set_state(gst::State::Null).map_err(|e| {
            VideoDecodingError(format!(
                "Failed to close video graciously ({path:?}):\n{e:#?}"
            ))
        })?;

        Ok(Stream {
            uri,
            source,
            bus,
            app_sink: None,
            frame_size: [width as u32, height as u32],
            frame_rate,
            audio_chan,
            audio_rate,
            duration,
            is_eos: false,
            paused: true,
        })
    }

    /// Get the size/resolution of the video as `[width, height]`.
    #[inline(always)]
    pub fn size(&self) -> [u32; 2] {
        self.frame_size
    }

    /// Get the framerate of the video as frames per second.
    #[inline(always)]
    pub fn framerate(&self) -> f64 {
        self.frame_rate
    }

    /// Get the number of audio channels.
    #[inline(always)]
    pub fn channels(&self) -> u16 {
        self.audio_chan
    }

    /// Check if stream has a video channel.
    #[inline(always)]
    pub fn has_video(&self) -> bool {
        self.frame_size.iter().sum::<u32>() > 0
    }

    /// Check if stream has an audio channel.
    #[inline(always)]
    pub fn has_audio(&self) -> bool {
        self.audio_chan > 0
    }

    /// Set the volume multiplier of the audio.
    /// `0.0` = 0% volume, `1.0` = 100% volume.
    pub fn set_volume(&mut self, volume: f64) {
        self.source.set_property("volume", &volume);
    }

    /// Set if the audio is muted or not, without changing the volume.
    pub fn set_muted(&mut self, muted: bool) {
        self.source.set_property("mute", &muted);
    }

    /// Get if the stream ended or not.
    #[inline(always)]
    pub fn eos(&self) -> bool {
        self.is_eos
    }

    /// Get if the stream is paused.
    #[inline(always)]
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Set if the media is paused or not.
    pub fn set_paused(&mut self, paused: bool) -> Result<(), error::Error> {
        self.source
            .set_state(if paused {
                gst::State::Paused
            } else {
                gst::State::Playing
            })
            .map_err(|e| VideoDecodingError(format!("Failed to change video state:\n{e:#?}")))?;
        self.paused = paused;
        Ok(())
    }

    /// Get the media duration.
    #[inline(always)]
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Process stream bus to see if stream has ended
    pub fn process_bus(&mut self, looping: bool) -> Result<bool, error::Error> {
        let mut eos = false;
        for msg in self.bus.iter() {
            match msg.view() {
                gst::MessageView::Error(e) => Err(VideoDecodingError(format!(
                    "Encountered error in gstreamer bus:\n{e:#?}"
                )))?,
                gst::MessageView::Eos(_eos) => eos = true,
                _ => {}
            }
        }

        if eos && looping {
            self.restart_stream()?;
            Ok(false)
        } else if eos {
            self.is_eos = true;
            self.set_paused(true)?;
            Ok(true)
        } else {
            Ok(self.is_eos)
        }
    }

    /// Starts a stream; assumes it is at first frame and unpauses.
    pub fn start_stream(&mut self) -> Result<(), error::Error> {
        self.set_paused(false)?;
        Ok(())
    }

    /// Restarts a stream; seeks to the first frame and unpauses, sets the `eos` flag to false.
    pub fn restart_stream(&mut self) -> Result<(), error::Error> {
        self.is_eos = false;
        let position: gst::GenericFormattedValue = gst::format::Default(0).into();
        self.source
            .seek_simple(gst::SeekFlags::FLUSH, position)
            .map_err(|e| VideoDecodingError(format!("Failed to seek video position:\n{e:#?}")))?;
        self.set_paused(false)?;
        Ok(())
    }

    pub fn set_sink_callback(
        &mut self,
        frame: Arc<Mutex<Option<image::Handle>>>,
    ) -> Result<(), error::Error> {
        if let Some(app_sink) = self.app_sink.take() {
            let [width, height] = self.size();

            thread::spawn(move || {
                app_sink.set_callbacks(
                    gst_app::AppSinkCallbacks::builder()
                        .new_sample(move |sink| {
                            let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                            let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                            let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;

                            *frame.lock().map_err(|_| gst::FlowError::Error)? =
                                Some(image::Handle::from_pixels(
                                    width,
                                    height,
                                    map.as_slice().to_owned(),
                                ));

                            Ok(gst::FlowSuccess::Ok)
                        })
                        .build(),
                );
            });
            Ok(())
        } else {
            Err(InternalError(
                "Tried to set app_sink on video twice".to_owned(),
            ))
        }
    }

    pub fn cloned(&self, volume: Option<f32>) -> Result<Self, error::Error> {
        let source = gst_launch(&self.uri, false, volume)?;
        let bus = source.bus().unwrap();

        let video_sink = video_sink_from_source(&source, true);
        if let Some(sink) = video_sink.as_ref() {
            sink.set_max_buffers(5 * self.frame_rate.ceil() as u32);
        }

        // let audio_sink = audio_sink_from_source(&source, true);
        // if let Some(sink) = audio_sink.as_ref() {
        //     sink.set_max_buffers(5 * self.audio_rate * 2);
        // }

        Ok(Stream {
            uri: self.uri.clone(),
            source,
            bus,
            app_sink: video_sink,
            frame_size: self.frame_size,
            frame_rate: self.frame_rate,
            audio_chan: self.audio_chan,
            audio_rate: self.audio_rate,
            duration: self.duration,
            is_eos: self.is_eos,
            paused: self.paused,
        })
    }

    pub fn pull_samples(&self) -> Result<(FrameBuffer, f64), error::Error> {
        let source = gst_launch(&self.uri, true, None)?;

        let video_sink = video_sink_from_source(&source, false);
        if let Some(sink) = video_sink.as_ref() {
            sink.set_max_lateness(0);
            sink.set_max_buffers(5 * self.frame_rate.ceil() as u32);
        }

        let audio_sink = audio_sink_from_source(&source, false);
        if let Some(sink) = audio_sink.as_ref() {
            sink.set_max_buffers(5 * self.audio_rate * 2);
        }

        source.set_property("mute", true);
        source.set_state(gst::State::Playing).map_err(|e| {
            VideoDecodingError(format!(
                "Failed to change video state (\"{}\"):\n{e:#?}",
                self.uri
            ))
        })?;

        let video_sink = video_sink
            .ok_or_else(|| InternalError("Tried to pull on non-existent video sink".to_owned()))?;

        let mut frames = vec![];
        let t1 = Instant::now();
        while let Ok(sample) = video_sink.pull_sample() {
            let buffer = sample.buffer().ok_or_else(|| {
                VideoDecodingError(format!(
                    "Failed to obtain buffer on video sample (\"{}\").",
                    self.uri
                ))
            })?;
            let map = buffer.map_readable().map_err(|e| {
                VideoDecodingError(format!(
                    "Failed to obtain map on buffered sample (\"{}\"):\n{e:#?}",
                    self.uri
                ))
            })?;

            frames.push(image::Handle::from_pixels(
                self.frame_size[0] as _,
                self.frame_size[1] as _,
                map.as_slice().to_owned(),
            ));
        }
        println!("Took {:?} to pull samples for video.", Instant::now() - t1);

        Ok((Arc::new(frames), self.frame_rate))
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Glib(#[from] glib::Error),
    #[error("{0}")]
    Bool(#[from] glib::BoolError),
    #[error("failed to get the gstreamer bus")]
    Bus,
    #[error("{0}")]
    StateChange(#[from] gst::StateChangeError),
    #[error("failed to cast gstreamer element")]
    Cast,
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("invalid URI")]
    Uri,
    #[error("failed to get media capabilities")]
    Caps,
    #[error("failed to query media duration or position")]
    Duration,
    #[error("failed to sync with playback")]
    Sync,
}

impl From<Error> for error::Error {
    fn from(e: Error) -> Self {
        VideoDecodingError(format!("{e:#?}"))
    }
}

fn gst_pipeline(uri: &str, audio: bool) -> String {
    if audio {
        format!(
            "playbin uri=\"{uri}\" \
            video-sink=\"videoconvert ! videoscale ! appsink name=video_sink caps=video/x-raw,format=BGRA,pixel-aspect-ratio=1/1\" \
            audio-sink=\"audioconvert ! appsink name=audio_sink caps=audio/x-raw,format=S16LE,layout=interleaved\""
        )
    } else {
        format!(
            "playbin uri=\"{uri}\" \
            video-sink=\"videoconvert ! videoscale ! appsink name=video_sink caps=video/x-raw,format=BGRA,pixel-aspect-ratio=1/1\""
        )
    }
}

fn gst_launch(uri: &str, audio: bool, volume: Option<f32>) -> Result<gst::Bin, error::Error> {
    let source = gst::parse_launch(&gst_pipeline(uri, audio)).map_err(|e| {
        VideoDecodingError(format!(
            "Failed to parse gstreamer command for video ({uri:?}):\n{e:#?}"
        ))
    })?;
    let source = source.downcast::<gst::Bin>().unwrap();

    if let Some(volume) = volume {
        source.set_property("volume", volume as f64);
    }

    source.set_state(gst::State::Paused).map_err(|e| {
        VideoDecodingError(format!(
            "Failed to change state for video ({uri:?}):\n{e:#?}"
        ))
    })?;
    source
        .state(gst::ClockTime::from_seconds(5))
        .0
        .map_err(|e| {
            VideoDecodingError(format!("Failed to read state for video ({uri:?}):\n{e:#?}"))
        })?;

    Ok(source)
}

fn video_sink_from_source(source: &gst::Bin, sync: bool) -> Option<gst_app::AppSink> {
    let video_sink: gst::Element = source.property("video-sink");
    let pad = video_sink.pads().get(0).cloned().unwrap();
    let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
    let bin = pad.parent_element().unwrap();
    let bin = bin.downcast::<gst::Bin>().unwrap();

    let app_sink = bin.by_name("video_sink").unwrap();
    let app_sink = app_sink.downcast::<gst_app::AppSink>().unwrap();
    app_sink.set_async(true);
    app_sink.set_sync(sync);
    app_sink.set_max_lateness(0);
    app_sink.set_max_buffers(10);

    if app_sink.pull_preroll().is_ok() {
        Some(app_sink)
    } else {
        None
    }
}

fn audio_sink_from_source(source: &gst::Bin, sync: bool) -> Option<gst_app::AppSink> {
    let audio_sink: gst::Element = source.property("audio-sink");
    let pad = audio_sink.pads().get(0).cloned().unwrap();
    let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
    let bin = pad.parent_element().unwrap();
    let bin = bin.downcast::<gst::Bin>().unwrap();

    let app_sink = bin.by_name("audio_sink").unwrap();
    let app_sink = app_sink.downcast::<gst_app::AppSink>().unwrap();
    app_sink.set_async(true);
    app_sink.set_sync(sync);
    app_sink.set_max_lateness(0);
    app_sink.set_max_buffers(10);

    if app_sink.pull_preroll().is_ok() {
        Some(app_sink)
    } else {
        None
    }
}

fn video_meta_from_sink(video_sink: &gst_app::AppSink) -> Result<(u32, u32, f64), error::Error> {
    let pad = video_sink.static_pad("sink").ok_or(Error::Caps)?;
    let caps = pad.current_caps().ok_or(Error::Caps)?;
    let caps = caps.structure(0).ok_or(Error::Caps)?;
    let width = caps.get::<i32>("width").map_err(|_| Error::Caps)? as u32;
    let height = caps.get::<i32>("height").map_err(|_| Error::Caps)? as u32;
    let video_rate = caps
        .get::<gst::Fraction>("framerate")
        .map_err(|_| Error::Caps)?;
    let video_rate = Rational32::new(video_rate.numer() as _, video_rate.denom() as _)
        .to_f64()
        .unwrap();
    Ok((width, height, video_rate))
}

fn audio_meta_from_sink(audio_sink: &gst_app::AppSink) -> Result<(u16, u32), error::Error> {
    let pad = audio_sink.static_pad("sink").ok_or(Error::Caps)?;
    let caps = pad.current_caps().ok_or(Error::Caps)?;
    let caps = caps.structure(0).ok_or(Error::Caps)?;
    let channels = caps.get::<i32>("channels").map_err(|_| Error::Caps)? as u16;
    let audio_rate = caps.get::<i32>("rate").map_err(|_| Error::Caps)? as u32;
    Ok((channels, audio_rate))
}
