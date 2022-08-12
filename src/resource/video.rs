use crate::config::Config;
use crate::error;
use crate::error::Error::{InternalError, ResourceLoadError, VideoDecodingError};
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;
use iced::pure::widget::image;
use num_rational::Rational32;
use num_traits::ToPrimitive;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{env, thread};
use thiserror::Error;

pub fn gst_init() -> Result<(), error::Error> {
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

    gst::init().map_err(|e| {
        VideoDecodingError(format!(
            "Failed to initialize GStreamer: required because there is a video element in this block:\n{e:#?}",
        ))
    })
}

pub fn video_from_file(path: &Path, _config: &Config) -> Result<Handle, error::Error> {
    Handle::new(path)
}

/// A video handle that uses GStreamer to stream video content.
/// This `struct` and its associated `impl` is a simplified version of the
/// `VideoPlayer` struct found at: https://github.com/jazzfool/iced_video_player.
pub struct Handle {
    uri: String,
    source: gst::Bin,
    bus: gst::Bus,
    app_sink: Option<gst_app::AppSink>,
    size: [u32; 2],
    framerate: f64,
    duration: Duration,
    is_eos: bool,
    paused: bool,
}

impl Debug for Handle {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[video::Handle]")
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.source
            .set_state(gst::State::Null)
            .expect("Failed to drop video handle");
    }
}

impl Handle {
    /// Create a new video object from a given video file.
    pub fn new(path: &Path) -> Result<Self, error::Error> {
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

        let source = gst::parse_launch(&format!(
            "playbin uri=\"{}\" video-sink=\"videoconvert ! videoscale ! appsink \
            name=app_sink caps=video/x-raw,format=BGRA,pixel-aspect-ratio=1/1,control-rate=2\"",
            uri
        ))
        .map_err(|e| {
            VideoDecodingError(format!(
                "Failed to parse gstreamer command for video ({path:?}):\n{e:#?}"
            ))
        })?
        .downcast::<gst::Bin>()
        .unwrap();

        let bus = source.bus().unwrap();

        let video_sink: gst::Element = source.property("video-sink");
        let pad = video_sink.pads().get(0).cloned().unwrap();
        let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
        let bin = pad.parent_element().unwrap();
        let bin = bin.downcast::<gst::Bin>().unwrap();

        let app_sink = bin.by_name("app_sink").unwrap();
        let app_sink = app_sink.downcast::<gst_app::AppSink>().unwrap();

        let frame = Arc::new(Mutex::new(None));
        set_sink_callback(app_sink, frame);

        source.set_state(gst::State::Playing).map_err(|e| {
            VideoDecodingError(format!(
                "Failed to change state for video ({path:?}):\n{e:#?}"
            ))
        })?;

        // wait for up to 5 seconds until the decoder gets the source capabilities
        source
            .state(gst::ClockTime::from_seconds(5))
            .0
            .map_err(|e| {
                VideoDecodingError(format!(
                    "Failed to read state for video ({path:?}):\n{e:#?}"
                ))
            })?;

        // extract resolution and framerate
        let caps = pad.current_caps().ok_or(Error::Caps)?;
        let s = caps.structure(0).ok_or(Error::Caps)?;
        let width = s.get::<i32>("width").map_err(|_| Error::Caps)?;
        let height = s.get::<i32>("height").map_err(|_| Error::Caps)?;
        let size = [width as u32, height as u32];
        let framerate = s
            .get::<gst::Fraction>("framerate")
            .map_err(|_| Error::Caps)?;
        let framerate = Rational32::new(framerate.numer() as _, framerate.denom() as _)
            .to_f64()
            .unwrap();

        let duration = Duration::from_nanos(
            source
                .query_duration::<gst::ClockTime>()
                .ok_or(Error::Duration)
                .unwrap()
                .nseconds(),
        );

        let position: gst::GenericFormattedValue = gst::format::Default(0).into();
        source.set_state(gst::State::Paused).map_err(|e| {
            VideoDecodingError(format!("Failed to pause video ({path:?}):\n{e:#?}"))
        })?;
        source
            .seek_simple(gst::SeekFlags::FLUSH, position)
            .map_err(|e| {
                VideoDecodingError(format!(
                    "Failed to seek position for video ({path:?}):\n{e:#?}"
                ))
            })?;

        Ok(Handle {
            uri,
            source,
            bus,
            app_sink: None,
            size,
            framerate,
            duration,
            is_eos: false,
            paused: true,
        })
    }

    /// Get the size/resolution of the video as `[width, height]`.
    #[inline(always)]
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    /// Get the framerate of the video as frames per second.
    #[inline(always)]
    pub fn framerate(&self) -> f64 {
        self.framerate
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
        frame_ref: Arc<Mutex<Option<image::Handle>>>,
    ) -> Result<(), error::Error> {
        if let Some(app_sink) = self.app_sink.take() {
            set_sink_callback(app_sink, frame_ref);
            Ok(())
        } else {
            Err(InternalError(
                "Tried to set app_sink on video twice".to_owned(),
            ))
        }
    }

    pub fn cloned(&self) -> Result<Self, error::Error> {
        let source = gst::parse_launch(&format!(
            "playbin uri=\"{}\" video-sink=\"videoconvert ! videoscale ! appsink \
            name=app_sink caps=video/x-raw,format=BGRA,pixel-aspect-ratio=1/1,control-rate=4\"",
            self.uri
        ))
        .map_err(|e| {
            VideoDecodingError(format!(
                "Failed to parse gstreamer command for video ({:?}):\n{e:#?}",
                self.uri
            ))
        })?
        .downcast::<gst::Bin>()
        .unwrap();

        let bus = source.bus().unwrap();

        let video_sink: gst::Element = source.property("video-sink");
        let pad = video_sink.pads().get(0).cloned().unwrap();
        let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
        let bin = pad.parent_element().unwrap();
        let bin = bin.downcast::<gst::Bin>().unwrap();

        let app_sink = bin.by_name("app_sink").unwrap();
        let app_sink = app_sink.downcast::<gst_app::AppSink>().unwrap();
        app_sink.set_sync(true);
        app_sink.set_max_lateness(0);
        app_sink.set_max_buffers(1000);
        source.set_state(gst::State::Paused).map_err(|e| {
            VideoDecodingError(format!(
                "Failed to initialize video state to Paused on video ({:?}):\n{e:#?}",
                self.uri
            ))
        })?;
        source
            .state(gst::ClockTime::from_seconds(5))
            .0
            .map_err(|e| {
                VideoDecodingError(format!(
                    "Failed to read state for video ({:?}):\n{e:#?}",
                    self.uri
                ))
            })?;
        app_sink.pull_preroll().map_err(|e| {
            VideoDecodingError(format!(
                "Failed to pull PREROLL sample from video ({:?}):\n{e:#?}",
                self.uri
            ))
        })?;

        Ok(Handle {
            uri: self.uri.clone(),
            source,
            bus,
            app_sink: Some(app_sink),
            size: self.size,
            framerate: self.framerate,
            duration: self.duration,
            is_eos: self.is_eos,
            paused: self.paused,
        })
    }

    pub fn pull_samples(&self) -> Result<(Vec<image::Handle>, f64), error::Error> {
        let source = gst::parse_launch(&format!(
            "playbin uri=\"{}\" video-sink=\"videoconvert ! videoscale ! appsink \
            name=app_sink caps=video/x-raw,format=BGRA,pixel-aspect-ratio=1/1,control-rate=4\"",
            self.uri
        ))
        .map_err(|e| {
            VideoDecodingError(format!(
                "Failed to parse gstreamer command for video ({:?}):\n{e:#?}",
                self.uri
            ))
        })?
        .downcast::<gst::Bin>()
        .unwrap();

        let video_sink: gst::Element = source.property("video-sink");
        let pad = video_sink.pads().get(0).cloned().unwrap();
        let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
        let bin = pad.parent_element().unwrap();
        let bin = bin.downcast::<gst::Bin>().unwrap();

        let app_sink = bin.by_name("app_sink").unwrap();
        let app_sink = app_sink.downcast::<gst_app::AppSink>().unwrap();
        app_sink.set_sync(true);
        app_sink.set_max_lateness(0);
        app_sink.set_max_buffers(1000);
        source.set_state(gst::State::Paused).map_err(|e| {
            VideoDecodingError(format!(
                "Failed to initialize video state to Paused on video ({:?}):\n{e:#?}",
                self.uri
            ))
        })?;
        source
            .state(gst::ClockTime::from_seconds(5))
            .0
            .map_err(|e| {
                VideoDecodingError(format!(
                    "Failed to read state for video ({:?}):\n{e:#?}",
                    self.uri
                ))
            })?;
        app_sink.pull_preroll().map_err(|e| {
            VideoDecodingError(format!(
                "Failed to pull PREROLL sample from video ({:?}):\n{e:#?}",
                self.uri
            ))
        })?;

        app_sink.set_sync(false);
        source.set_property("mute", true);
        source.set_state(gst::State::Playing).map_err(|e| {
            VideoDecodingError(format!(
                "Failed to change video state (\"{}\"):\n{e:#?}",
                self.uri
            ))
        })?;

        let mut frames = vec![];
        let t1 = Instant::now();
        while let Ok(sample) = app_sink.pull_sample() {
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
                self.size[0] as _,
                self.size[1] as _,
                map.as_slice().to_owned(),
            ));
        }
        println!("Took {:?} to pull samples for video.", Instant::now() - t1);

        Ok((frames, self.framerate))
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

fn set_sink_callback(app_sink: gst_app::AppSink, frame_ref: Arc<Mutex<Option<image::Handle>>>) {
    thread::spawn(move || {
        app_sink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                    let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;

                    let pad = sink.static_pad("sink").ok_or(gst::FlowError::Error)?;

                    let caps = pad.current_caps().ok_or(gst::FlowError::Error)?;
                    let caps = caps.structure(0).ok_or(gst::FlowError::Error)?;
                    let width = caps
                        .get::<i32>("width")
                        .map_err(|_| gst::FlowError::Error)?;
                    let height = caps
                        .get::<i32>("height")
                        .map_err(|_| gst::FlowError::Error)?;

                    *frame_ref.lock().map_err(|_| gst::FlowError::Error)? =
                        Some(image::Handle::from_pixels(
                            width as _,
                            height as _,
                            map.as_slice().to_owned(),
                        ));

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );
    });
}
