use crate::backend::{MediaMode, MediaStream};
use crate::config::Config;
use crate::error;
use crate::error::Error::{
    BackendError, InternalError, InvalidConfigError, ResourceLoadError, StreamDecodingError,
};
use crate::resource::FrameBuffer;
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;
use iced::pure::widget::image;
use num_rational::Rational32;
use num_traits::ToPrimitive;
use once_cell::sync::OnceCell;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{env, thread};
use thiserror::Error;

static GST_INIT: OnceCell<()> = OnceCell::new();

/// A video handle that uses GStreamer to stream video content.
/// This `struct` and its associated `impl` is a simplified version of the
/// `VideoPlayer` struct found at: https://github.com/jazzfool/iced_video_player.
pub struct Stream {
    path: PathBuf,
    source: gst::Bin,
    playbin: gst::Bin,
    bus: gst::Bus,
    media_mode: MediaMode,
    frame_size: [u32; 2],
    frame_rate: f64,
    audio_chan: u16,
    audio_rate: u32,
    duration: Duration,
    is_eos: bool,
    paused: bool,
}

impl MediaStream for Stream {
    /// Create a new video object from a given video file.
    fn new(path: &Path, _config: &Config, media_mode: MediaMode) -> Result<Self, error::Error> {
        init()?;

        let (source, playbin) = launch(path, &MediaMode::Query, None)?;
        let bus = source.bus().unwrap();

        let video_sink = get_video_sink(&playbin, false);
        let (width, height, frame_rate) = match video_sink.as_ref() {
            Some(sink) => video_meta_from_sink(sink)?,
            None => (0, 0, 0.0),
        };

        let audio_sink = get_audio_sink(&playbin, false);
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
            StreamDecodingError(format!(
                "Failed to close video graciously ({path:?}):\n{e:#?}"
            ))
        })?;

        let (media_mode, audio_chan) = match (media_mode, audio_chan) {
            (MediaMode::SansIntTrigger, 0) => Err(InvalidConfigError(format!(
                "Cannot assume integrated trigger due to missing audio stream: {path:?}"
            ))),
            (MediaMode::SansIntTrigger, 1) => Ok((MediaMode::Muted, 0)),
            (MediaMode::SansIntTrigger, 2) => Ok((MediaMode::SansIntTrigger, 1)),
            (MediaMode::SansIntTrigger, _) => Err(InvalidConfigError(format!(
                "Cannot use integrated trigger with multichannel (n = {audio_chan} > 2) audio: {path:?}"
            ))),
            (MediaMode::WithExtTrigger(t), c @ 0..=1) => Ok((MediaMode::WithExtTrigger(t), c)),
            (MediaMode::WithExtTrigger(_), c) if c > 1 => Err(InvalidConfigError(format!(
                "Cannot add trigger stream to non-mono (n = {audio_chan}) audio stream: {path:?}"
            ))),
            (mode, c) => Ok((mode, c)),
        }?;

        Ok(Stream {
            path: path.to_owned(),
            source,
            playbin,
            bus,
            media_mode,
            frame_size: [width as u32, height as u32],
            frame_rate,
            audio_chan,
            audio_rate,
            duration,
            is_eos: false,
            paused: true,
        })
    }

    fn cloned(
        &self,
        frame: Arc<Mutex<Option<image::Handle>>>,
        volume: Option<f32>,
    ) -> Result<Self, error::Error> {
        let (source, playbin) = launch(&self.path, &self.media_mode, volume)?;
        let bus = source.bus().unwrap();

        let video_sink = get_video_sink(&playbin, true);
        if let Some(sink) = video_sink {
            sink.set_max_buffers(5 * self.frame_rate.ceil() as u32);
            let [width, height] = self.size();

            thread::spawn(move || {
                sink.set_callbacks(
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
        }

        Ok(Stream {
            path: self.path.clone(),
            source,
            playbin,
            bus,
            media_mode: self.media_mode.clone(),
            frame_size: self.frame_size,
            frame_rate: self.frame_rate,
            audio_chan: self.audio_chan,
            audio_rate: self.audio_rate,
            duration: self.duration,
            is_eos: self.is_eos,
            paused: self.paused,
        })
    }

    /// Get if the stream ended or not.
    #[inline(always)]
    fn eos(&self) -> bool {
        self.is_eos
    }

    /// Get the size/resolution of the video as `[width, height]`.
    #[inline(always)]
    fn size(&self) -> [u32; 2] {
        self.frame_size
    }

    /// Get the framerate of the video as frames per second.
    #[inline(always)]
    fn framerate(&self) -> f64 {
        self.frame_rate
    }

    /// Get the number of audio channels.
    #[inline(always)]
    fn channels(&self) -> u16 {
        self.audio_chan
    }

    /// Get the media duration.
    #[inline(always)]
    fn duration(&self) -> Duration {
        self.duration
    }

    /// Check if stream has a video channel.
    #[inline(always)]
    fn has_video(&self) -> bool {
        self.frame_size.iter().sum::<u32>() > 0
    }

    /// Check if stream has an audio channel.
    #[inline(always)]
    fn has_audio(&self) -> bool {
        self.audio_chan > 0
    }

    /// Starts a stream; assumes it is at first frame and unpauses.
    fn start(&mut self) -> Result<(), error::Error> {
        self.set_paused(false)?;
        Ok(())
    }

    /// Restarts a stream; seeks to the first frame and unpauses, sets the `eos` flag to false.
    fn restart(&mut self) -> Result<(), error::Error> {
        self.is_eos = false;
        let position: gst::GenericFormattedValue = gst::format::Default(0).into();
        self.source
            .seek_simple(gst::SeekFlags::FLUSH, position)
            .map_err(|e| StreamDecodingError(format!("Failed to seek video position:\n{e:#?}")))?;
        self.set_paused(false)?;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), error::Error> {
        self.set_paused(true)
    }

    fn pull_samples(&self) -> Result<(FrameBuffer, f64), error::Error> {
        let (source, playbin) = launch(&self.path, &MediaMode::Query, None)?;

        let video_sink = get_video_sink(&playbin, false);
        if let Some(sink) = video_sink.as_ref() {
            sink.set_max_lateness(0);
            sink.set_max_buffers(5 * self.frame_rate.ceil() as u32);
        }

        let audio_sink = get_audio_sink(&playbin, false);
        if let Some(sink) = audio_sink.as_ref() {
            sink.set_max_buffers(5 * self.audio_rate * 2);
        }

        playbin.set_property("mute", true);
        source.set_state(gst::State::Playing).map_err(|e| {
            StreamDecodingError(format!(
                "Failed to change video state (\"{:?}\"):\n{e:#?}",
                self.path
            ))
        })?;

        let video_sink = video_sink
            .ok_or_else(|| InternalError("Tried to pull on non-existent video sink".to_owned()))?;

        let mut frames = vec![];
        let t1 = Instant::now();
        while let Ok(sample) = video_sink.pull_sample() {
            let buffer = sample.buffer().ok_or_else(|| {
                StreamDecodingError(format!(
                    "Failed to obtain buffer on video sample (\"{:?}\").",
                    self.path
                ))
            })?;
            let map = buffer.map_readable().map_err(|e| {
                StreamDecodingError(format!(
                    "Failed to obtain map on buffered sample (\"{:?}\"):\n{e:#?}",
                    self.path
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

    /// Process stream bus to see if stream has ended
    fn process_bus(&mut self, looping: bool) -> Result<bool, error::Error> {
        let mut eos = false;
        for msg in self.bus.iter() {
            match msg.view() {
                gst::MessageView::Error(e) => Err(StreamDecodingError(format!(
                    "Encountered error in gstreamer bus:\n{e:#?}"
                )))?,
                gst::MessageView::Eos(_eos) => eos = true,
                _ => {}
            }
        }

        if eos && looping {
            self.restart()?;
            Ok(false)
        } else if eos {
            self.is_eos = true;
            self.set_paused(true)?;
            Ok(true)
        } else {
            Ok(self.is_eos)
        }
    }
}

impl Stream {
    /// Set the volume multiplier of the audio.
    /// `0.0` = 0% volume, `1.0` = 100% volume.
    pub fn set_volume(&mut self, volume: f64) {
        self.playbin.set_property("volume", &volume);
    }

    /// Set if the audio is muted or not, without changing the volume.
    pub fn set_muted(&mut self, muted: bool) {
        self.playbin.set_property("mute", &muted);
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
            .map_err(|e| StreamDecodingError(format!("Failed to change video state:\n{e:#?}")))?;
        self.paused = paused;
        Ok(())
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.source
            .set_state(gst::State::Null)
            .expect("Failed to drop video handle");
    }
}

pub fn init() -> Result<(), error::Error> {
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
            BackendError(format!(
                "Failed to initialize GStreamer: required because there is a video element \
                in this block:\n{e:#?}",
            ))
        })
}

fn pipeline(path: &Path, mode: &MediaMode) -> Result<String, error::Error> {
    let mut pipeline = format!(
        "\
        playbin uri=\"file://{}\" name=playbin \
        video-sink=\"videoconvert ! videoscale ! appsink name=video_sink caps=video/x-raw,format=BGRA,pixel-aspect-ratio=1/1\"",
        path.canonicalize()
            .map_err(|e| ResourceLoadError(format!(
                "Failed to canonicalize resource path: {path:?}\n{e:#?}"
            )))?
            .to_str()
            .unwrap()
    );

    match mode {
        MediaMode::Query => pipeline.push_str(
            " \
            audio-sink=\"audioconvert ! appsink name=audio_sink caps=audio/x-raw,format=S16LE,layout=interleaved\""
        ),
        MediaMode::Normal => {},
        MediaMode::Muted => pipeline.push_str(
            " \
            audio-sink=\"audioconvert ! fakesink\""
        ),
        MediaMode::SansIntTrigger => pipeline.push_str(
            " \
            audio-sink=\"audioconvert ! deinterleave name=d ! d.src_0 ! playsink\""
        ),
        MediaMode::WithExtTrigger(trigger) => write!(
            pipeline,
            " \
            audio-sink=\"audioconvert ! audiopanorama panorama=-1 ! playsink\" \
            uridecodebin uri=\"file://{}\" ! audioconvert ! audiopanorama panorama=1 ! playsink",
            trigger
                .canonicalize()
                .map_err(|e| ResourceLoadError(format!(
                        "Failed to canonicalize trigger path: {trigger:?}\n{e:#?}"
                )))?
                .to_str()
                .unwrap()
        ).unwrap(),
    };

    Ok(pipeline)
}

fn launch(
    path: &Path,
    mode: &MediaMode,
    volume: Option<f32>,
) -> Result<(gst::Bin, gst::Bin), error::Error> {
    let source = gst::parse_launch(&pipeline(path, mode)?)
        .map_err(|e| {
            StreamDecodingError(format!(
                "Failed to parse gstreamer command for video ({path:?}):\n{e:#?}"
            ))
        })?
        .downcast::<gst::Bin>()
        .unwrap();

    let playbin = if matches!(mode, MediaMode::WithExtTrigger(_)) {
        source
            .by_name("playbin")
            .unwrap()
            .downcast::<gst::Bin>()
            .unwrap()
    } else {
        source.clone()
    };

    if let Some(volume) = volume {
        playbin.set_property("volume", volume as f64);
    }

    source.set_state(gst::State::Paused).map_err(|e| {
        StreamDecodingError(format!(
            "Failed to change state for video ({path:?}):\n{e:#?}"
        ))
    })?;
    source
        .state(gst::ClockTime::from_seconds(5))
        .0
        .map_err(|e| {
            StreamDecodingError(format!(
                "Failed to read state for video ({path:?}):\n{e:#?}"
            ))
        })?;

    Ok((source, playbin))
}

fn get_video_sink(source: &gst::Bin, sync: bool) -> Option<gst_app::AppSink> {
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

    let timeout = gst::ClockTime::from_seconds(5);
    if app_sink.try_pull_preroll(Some(timeout)).is_some() {
        Some(app_sink)
    } else {
        None
    }
}

fn get_audio_sink(source: &gst::Bin, sync: bool) -> Option<gst_app::AppSink> {
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

    let timeout = gst::ClockTime::from_seconds(5);
    if app_sink.try_pull_preroll(Some(timeout)).is_some() {
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
    #[error("failed to get media capabilities")]
    Caps,
    #[error("failed to query media duration or position")]
    Duration,
    #[error("failed to sync with playback")]
    Sync,
}

impl From<Error> for error::Error {
    fn from(e: Error) -> Self {
        StreamDecodingError(format!("{e:#?}"))
    }
}
