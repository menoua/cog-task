use crate::backend::{MediaMode, MediaStream};
use crate::config::Config;
use crate::error;
use crate::error::Error::{BackendError, InternalError, InvalidConfigError, StreamDecodingError};
use crate::resource::FrameBuffer;
use crate::util::spin_sleeper;
use eframe::egui;
use eframe::egui::mutex::RwLock;
use eframe::egui::{ColorImage, ImageData, TextureFilter, TextureId, Vec2};
use eframe::epaint::TextureManager;
use ffmpeg::format::{context::Input, input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use ffmpeg_next as ffmpeg;
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{RecvError, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

static FFMPEG_INIT: OnceCell<()> = OnceCell::new();

#[derive(Clone)]
pub struct Stream {
    path: PathBuf,
    _context: Option<Arc<Mutex<Input>>>,
    video_index: Option<usize>,
    audio_index: Option<usize>,
    media_mode: MediaMode,
    frame_size: [u32; 2],
    frame_rate: f64,
    audio_chan: u16,
    audio_rate: u32,
    duration: Duration,
    is_eos: Arc<Mutex<bool>>,
    paused: bool,
    starter: Option<Sender<()>>,
    tex_manager: Arc<RwLock<TextureManager>>,
}

impl MediaStream for Stream {
    fn new(
        tex_manager: Arc<RwLock<TextureManager>>,
        path: &Path,
        _config: &Config,
        media_mode: MediaMode,
    ) -> Result<Self, error::Error> {
        init()?;

        let context = input(&path)?;

        let video = context.streams().best(Type::Video);
        let (video_index, width, height, frame_rate) = if let Some(stream) = video {
            let decoder = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?
                .decoder()
                .video()?;
            let rate = stream.avg_frame_rate();
            (
                Some(stream.index()),
                decoder.width(),
                decoder.height(),
                rate.numerator() as f64 / rate.denominator() as f64,
            )
        } else {
            (None, 0, 0, 0.0)
        };

        let audio = context.streams().best(Type::Audio);
        let (audio_index, audio_chan, audio_rate) = if let Some(stream) = audio {
            let decoder = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?
                .decoder()
                .audio()?;
            (Some(stream.index()), decoder.channels(), decoder.rate())
        } else {
            (None, 0, 0)
        };

        let duration = Duration::from_secs_f64(
            context.duration() as f64 / f64::from(ffmpeg::ffi::AV_TIME_BASE),
        );

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
            _context: None,
            video_index,
            audio_index,
            media_mode,
            frame_size: [width, height],
            frame_rate,
            audio_chan,
            audio_rate,
            duration,
            is_eos: Arc::new(Mutex::new(false)),
            paused: true,
            starter: None,
            tex_manager,
        })
    }

    fn cloned(
        &self,
        frame: Arc<Mutex<Option<(TextureId, Vec2)>>>,
        _volume: Option<f32>,
    ) -> Result<Self, error::Error> {
        let context = input(&self.path)?;
        // context
        //     .pause()
        //     .map_err(|e| InternalError(format!("{e:#?}")))?;

        let context = Arc::new(Mutex::new(context));
        let is_eos = Arc::new(Mutex::new(*self.is_eos.lock().unwrap()));
        let (tx_start, rx_start) = mpsc::channel();

        if let Some(index) = self.video_index {
            let path = self.path.clone();
            let framerate = self.frame_rate;
            let context = context.clone();
            let tex_manager = self.tex_manager.clone();
            let is_eos = is_eos.clone();

            thread::spawn(move || {
                let (mut decoder, mut scaler) = {
                    let context = context.lock().unwrap();
                    let stream = context.stream(index).expect("Failed to fetch video stream");

                    let decoder =
                        ffmpeg::codec::context::Context::from_parameters(stream.parameters())
                            .expect("Failed to create context for video stream")
                            .decoder()
                            .video()
                            .expect("Failed to decode video stream");

                    let scaler = Context::get(
                        decoder.format(),
                        decoder.width(),
                        decoder.height(),
                        Pixel::RGBA,
                        decoder.width(),
                        decoder.height(),
                        Flags::BILINEAR,
                    )
                    .expect("Failed to get context for decoded video stream");

                    (decoder, scaler)
                };

                let sleeper = spin_sleeper();

                if let Err(_) = rx_start.recv() {
                    *is_eos.lock().unwrap() = true;
                    return;
                }

                let dt = Duration::from_secs_f64(1.0 / framerate);
                let mut frame_start;

                loop {
                    frame_start = Instant::now();

                    {
                        let mut context = context.lock().unwrap();
                        let (stream, packet) = match context.packets().next() {
                            Some(packet) => packet,
                            None => break,
                        };
                        if stream.index() != index {
                            continue;
                        }

                        decoder
                            .send_packet(&packet)
                            .expect("Failed to send ffmpeg packet");

                        let mut decoded = Video::empty();
                        while decoder.receive_frame(&mut decoded).is_ok() {
                            let mut rgba_frame = Video::empty();
                            scaler
                                .run(&decoded, &mut rgba_frame)
                                .expect("Failed to run scaler");
                            *frame.lock().unwrap() = Some((
                                tex_manager.write().alloc(
                                    format!("{:?}:@:[current]", path),
                                    ImageData::Color(ColorImage::from_rgba_unmultiplied(
                                        [rgba_frame.width() as _, rgba_frame.height() as _],
                                        rgba_frame.data(0),
                                    )),
                                    TextureFilter::Linear,
                                ),
                                Vec2::new(rgba_frame.width() as _, rgba_frame.height() as _),
                            ));
                        }
                    }

                    let now = Instant::now();
                    sleeper.sleep(frame_start + dt - now);
                }

                *is_eos.lock().unwrap() = true;
                decoder
                    .send_eof()
                    .map_err(|e| {
                        StreamDecodingError(format!("Failed to send EOF to decoder: {e:?}"))
                    })
                    .unwrap();
            });
        }

        // if let Some(index) = self.video_index {
        //     let stream = context.stream(index).ok_or(InternalError(format!(
        //         "Failed to fetch audio stream for: {:?}",
        //         self.path
        //     )))?;
        //
        //     let mut decoder =
        //         ffmpeg::codec::context::Context::from_parameters(stream.parameters())?
        //             .decoder()
        //             .audio()
        //             .map_err(|e| {
        //                 InternalError(format!(
        //                     "Failed to decode audio stream for: {:?}",
        //                     self.path
        //                 ))
        //             })?;
        // }

        Ok(Stream {
            path: self.path.clone(),
            _context: Some(context),
            video_index: self.video_index,
            audio_index: self.audio_index,
            media_mode: self.media_mode.clone(),
            frame_size: self.frame_size,
            frame_rate: self.frame_rate,
            audio_chan: self.audio_chan,
            audio_rate: self.audio_rate,
            duration: self.duration,
            is_eos,
            paused: self.paused,
            starter: Some(tx_start),
            tex_manager: self.tex_manager.clone(),
        })
    }

    fn eos(&self) -> bool {
        *self.is_eos.lock().unwrap()
    }

    fn size(&self) -> [u32; 2] {
        self.frame_size
    }

    fn framerate(&self) -> f64 {
        self.frame_rate
    }

    fn channels(&self) -> u16 {
        self.audio_chan
    }

    fn duration(&self) -> Duration {
        self.duration
    }

    fn has_video(&self) -> bool {
        self.frame_size.iter().sum::<u32>() > 0
    }

    fn has_audio(&self) -> bool {
        self.audio_chan > 0
    }

    fn start(&mut self) -> Result<(), error::Error> {
        // self.context
        //     .as_ref()
        //     .unwrap()
        //     .lock()
        //     .unwrap()
        //     .play()
        //     .map_err(|e| {
        //         VideoDecodingError(format!(
        //             "Failed to start stream for: {:?}\n{e:#?}",
        //             self.path
        //         ))
        //     })?;
        self.paused = false;
        if let Some(link) = self.starter.take() {
            link.send(())
                .map_err(|e| InternalError("Failed to start ffmpeg parallel thread".to_owned()))
        } else {
            Err(InternalError(
                "Tried to start ffmpeg parallel thread twice".to_owned(),
            ))
        }
    }

    fn restart(&mut self) -> Result<(), error::Error> {
        // self.context.as_ref().unwrap().lock().unwrap().seek(0);
        Ok(())
    }

    fn pause(&mut self) -> Result<(), error::Error> {
        // self.context
        //     .as_ref()
        //     .unwrap()
        //     .lock()
        //     .unwrap()
        //     .pause()
        //     .map_err(|e| {
        //         VideoDecodingError(format!(
        //             "Failed to pause stream for: {:?}\n{e:#?}",
        //             self.path
        //         ))
        //     })?;
        self.paused = true;
        self._context.take();
        Ok(())
    }

    fn pull_samples(&self) -> Result<(FrameBuffer, f64), error::Error> {
        let index = self.video_index.ok_or_else(|| {
            InternalError(format!(
                "Tried to pull samples on non-video stream: {:?}",
                self.path
            ))
        })?;

        let mut context = input(&self.path)?;
        let (mut decoder, mut scaler) = {
            let stream = context.stream(index).expect("Failed to fetch video stream");

            let decoder = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
                .expect("Failed to create context for video stream")
                .decoder()
                .video()
                .expect("Failed to decode video stream");

            let scaler = Context::get(
                decoder.format(),
                decoder.width(),
                decoder.height(),
                Pixel::RGBA,
                decoder.width(),
                decoder.height(),
                Flags::BILINEAR,
            )
            .expect("Failed to get context for decoded video stream");

            (decoder, scaler)
        };

        let mut frames = vec![];
        for (stream, packet) in context.packets() {
            if stream.index() != index {
                continue;
            }

            decoder
                .send_packet(&packet)
                .expect("Failed to send ffmpeg packet");

            let mut decoded = Video::empty();
            while decoder.receive_frame(&mut decoded).is_ok() {
                let mut rgba_frame = Video::empty();
                scaler
                    .run(&decoded, &mut rgba_frame)
                    .expect("Failed to run scaler");
                // frames.push(image::Handle::from_pixels(
                //     bgra_frame.width(),
                //     bgra_frame.height(),
                //     bgra_frame.data(0).to_owned(),
                // ));
                frames.push((
                    self.tex_manager.write().alloc(
                        format!("{:?}:@:{}", self.path, frames.len()),
                        ImageData::Color(ColorImage::from_rgba_unmultiplied(
                            [rgba_frame.width() as _, rgba_frame.height() as _],
                            rgba_frame.data(0),
                        )),
                        TextureFilter::Linear,
                    ),
                    Vec2::new(rgba_frame.width() as _, rgba_frame.height() as _),
                ));
            }
        }

        decoder.send_eof().expect("Failed to send EOF to decoder");
        Ok((Arc::new(frames), self.frame_rate))
    }

    fn process_bus(&mut self, _looping: bool) -> Result<bool, error::Error> {
        Ok(self.eos())
    }
}

pub fn init() -> Result<(), error::Error> {
    if FFMPEG_INIT.get().is_some() {
        return Ok(());
    }

    ffmpeg::init()
        .map(|r| {
            FFMPEG_INIT.set(()).expect("Tried to init ffmpeg twice");
            r
        })
        .map_err(|e| {
            BackendError(format!(
                "Failed to initialize ffmpeg: required because there is a stream element \
                in this block:\n{e:#?}",
            ))
        })
}

impl From<ffmpeg::Error> for error::Error {
    fn from(e: ffmpeg::Error) -> Self {
        StreamDecodingError(format!("{e:#?}"))
    }
}
