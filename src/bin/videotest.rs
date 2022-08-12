use gst::element_error;
use gst::prelude::*;
use gst_app::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use std::env;
use std::env::args;
use std::io::{BufReader, Cursor};
use std::time::Instant;

use byte_slice_cast::*;

use anyhow::Error;
use derive_more::{Display, Error};
use glib::BoolError;
use gstreamer::{Sample, StateChangeError, StateChangeSuccess};
use iced::image;
use itertools::Itertools;
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, Source};

#[derive(Debug, Display, Error)]
#[display(fmt = "Missing element {}", _0)]
struct MissingElement(#[error(not(source))] &'static str);

#[derive(Debug, Display, Error)]
#[display(fmt = "Received error from {}: {} (debug: {:?})", src, error, debug)]
struct ErrorMessage {
    src: String,
    error: String,
    debug: Option<String>,
    source: glib::Error,
}

fn create_pipeline() -> Result<(), anyhow::Error> {
    gst::init()?;

    let args: Vec<_> = env::args().collect();
    let uri: &str = if args.len() == 2 {
        args[1].as_ref()
    } else {
        println!("Usage: decodebin file_path");
        std::process::exit(-1)
    };

    let source = gst::parse_launch(&format!(
        "playbin uri=file:///Users/menoua/CLionProjects/cog-task-rs/task/Dummy/data/video.mpg video-sink=\"videoconvert ! videoscale ! appsink \
        name=video_sink caps=video/x-raw,format=BGRA,pixel-aspect-ratio=1/1\" audio-sink=\"audioconvert ! appsink name=audio_sink caps=audio/x-raw,format=S16LE,layout=interleaved\""
    ))?
        .downcast::<gst::Bin>()
        .unwrap();

    let video_sink: gst::Element = source.property("video-sink");
    let pad = video_sink.pads().get(0).cloned().unwrap();
    let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
    let bin = pad.parent_element().unwrap();
    let bin = bin.downcast::<gst::Bin>().unwrap();
    let video_sink = bin.by_name("video_sink").unwrap();
    let video_sink = video_sink.downcast::<gst_app::AppSink>().unwrap();

    let audio_sink: gst::Element = source.property("audio-sink");
    let pad = audio_sink.pads().get(0).cloned().unwrap();
    let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
    let bin = pad.parent_element().unwrap();
    let bin = bin.downcast::<gst::Bin>().unwrap();
    let audio_sink = bin.by_name("audio_sink").unwrap();
    let audio_sink = audio_sink.downcast::<gst_app::AppSink>().unwrap();

    video_sink.set_sync(true);
    video_sink.set_max_lateness(0);
    audio_sink.set_sync(true);
    audio_sink.set_max_lateness(0);

    source.state(gst::ClockTime::from_seconds(5)).0?;
    source.set_state(gst::State::Paused)?;
    video_sink.pull_preroll()?;
    audio_sink.pull_preroll()?;

    let pad = video_sink.static_pad("sink").ok_or(gst::FlowError::Error)?;
    let caps = pad.current_caps().ok_or(gst::FlowError::Error)?;
    let caps = caps.structure(0).ok_or(gst::FlowError::Error)?;
    let width = caps
        .get::<i32>("width")
        .map_err(|_| gst::FlowError::Error)?;
    let height = caps
        .get::<i32>("height")
        .map_err(|_| gst::FlowError::Error)?;

    let pad = audio_sink.static_pad("sink").ok_or(gst::FlowError::Error)?;
    let caps = pad.current_caps().ok_or(gst::FlowError::Error)?;
    let caps = caps.structure(0).ok_or(gst::FlowError::Error)?;
    let rate = caps.get::<i32>("rate").map_err(|_| gst::FlowError::Error)?;
    let channels = caps
        .get::<i32>("channels")
        .map_err(|_| gst::FlowError::Error)?;

    video_sink.set_async(true);
    video_sink.set_sync(false);
    video_sink.set_max_buffers(2000);
    audio_sink.set_async(true);
    audio_sink.set_sync(false);
    audio_sink.set_max_buffers(2000);

    source.set_state(gst::State::Playing)?;

    let mut video_eos = false;
    let mut audio_eos = false;
    let mut video_frames = vec![];
    let mut audio_frames = vec![];
    let t1 = Instant::now();
    println!("Starting pull...");
    while !video_eos || !audio_eos {
        if !video_eos {
            match video_sink.pull_sample() {
                Ok(sample) => {
                    let buffer = sample.buffer().unwrap();
                    let map = buffer.map_readable().unwrap();
                    video_frames.push(image::Handle::from_pixels(
                        width as _,
                        height as _,
                        map.as_slice().to_owned(),
                    ));
                }
                Err(_) => {
                    video_eos = true;
                }
            }
        }

        if !audio_eos {
            match audio_sink.pull_sample() {
                Ok(sample) => {
                    let buffer = sample.buffer().unwrap();
                    let map = buffer.map_readable().unwrap();
                    audio_frames.extend_from_slice(map.as_slice_of::<i16>().unwrap());
                }
                Err(_) => {
                    audio_eos = true;
                }
            }
        }
    }
    source.set_state(gst::State::Null)?;
    println!("Took {:?} to pull samples for video.", Instant::now() - t1);

    let (a, h) = rodio::OutputStream::try_default().unwrap();
    let audio_src = SamplesBuffer::new(channels as u16, rate as u32, audio_frames);
    let audio_sink = rodio::Sink::try_new(&h).unwrap();

    println!("Starting audio playback...");
    let t1 = Instant::now();
    audio_sink.append(audio_src);
    audio_sink.sleep_until_end();
    println!("Took {:?} to play audio.", Instant::now() - t1);

    Ok(())
}

fn main() {
    create_pipeline().unwrap();
}
