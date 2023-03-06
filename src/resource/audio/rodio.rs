use crate::resource::AudioChannel;
use crate::server::Config;
use eyre::{eyre, Context, Result};
use rodio::buffer::SamplesBuffer;
use rodio::source::Buffered;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

#[derive(Clone)]
pub struct Buffer(Buffered<SamplesBuffer<i16>>);
pub struct Sink(rodio::Sink);
pub struct Device(OutputStream, OutputStreamHandle);

impl Device {
    pub fn new() -> Result<Self> {
        let (audio_stream, audio_stream_handle) =
            OutputStream::try_default().wrap_err("Failed to obtain audio output stream.")?;
        Ok(Self(audio_stream, audio_stream_handle))
    }

    pub fn sink(&self) -> Result<Sink> {
        let sink = rodio::Sink::try_new(&self.1)?;
        sink.pause();
        Ok(Sink(sink))
    }
}

impl Sink {
    #[inline(always)]
    pub fn pause(&self) {
        self.0.pause();
    }

    #[inline(always)]
    pub fn set_volume(&self, volume: f32) {
        self.0.set_volume(volume);
    }

    #[inline(always)]
    pub fn queue(&self, buffer: Buffer) {
        self.0.append(buffer.0);
    }

    #[inline(always)]
    pub fn repeat(&self, buffer: Buffer) {
        self.0.append(buffer.0.repeat_infinite());
    }

    #[inline(always)]
    pub fn play(&self) {
        self.0.play();
    }

    #[inline(always)]
    pub fn stop(&self) {
        self.0.stop();
    }

    #[inline(always)]
    pub fn empty(&self) -> bool {
        self.0.empty()
    }

    #[inline(always)]
    pub fn detach(self) {
        self.0.detach()
    }
}

impl Buffer {
    pub fn new(path: &Path, _config: &Config) -> Result<Self> {
        let decoder = Decoder::new(BufReader::new(
            File::open(path).wrap_err_with(|| format!("Failed to open audio file: {path:?}"))?,
        ))
        .wrap_err_with(|| format!("Failed to decode audio file: {path:?}"))?;

        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels() as i16;
        let samples: Vec<_> = decoder.collect();

        Ok(Self(
            SamplesBuffer::new(channels as u16, sample_rate, samples).buffered(),
        ))
    }

    #[inline(always)]
    pub fn duration(&self) -> Duration {
        self.0.total_duration().unwrap_or_default()
    }

    #[inline(always)]
    pub fn sample_rate(&self) -> u32 {
        self.0.sample_rate()
    }

    #[inline(always)]
    pub fn channels(&self) -> u16 {
        self.0.channels()
    }

    pub fn interlaced(mut self, mut other: Self) -> Result<Self> {
        let sample_rate = self.0.sample_rate();
        let in_channels = self.0.channels() as i16;
        let other_channels = other.0.channels() as i16;
        let out_channels = in_channels + other_channels;

        if other.0.sample_rate() != sample_rate {
            return Err(eyre!(
                "Cannot interlace audio buffers with different sampling rates: {}, {}",
                sample_rate,
                other.0.sample_rate(),
            ));
        }

        let mut c = -1;
        let mut samples = vec![];
        let mut status = 0;
        loop {
            c = (c + 1) % out_channels;
            if c == 0 && status == 3 {
                break;
            }

            samples.push(if c < in_channels {
                self.0.next().unwrap_or_else(|| {
                    status |= 1;
                    0
                })
            } else {
                other.0.next().unwrap_or_else(|| {
                    status |= 2;
                    0
                })
            });
        }

        Ok(Self(
            SamplesBuffer::new(out_channels as u16, sample_rate, samples).buffered(),
        ))
    }

    pub fn with_direction(self, channel: AudioChannel) -> Result<Self> {
        let sample_rate = self.sample_rate();
        let out_channels = match (self.channels(), channel) {
            (1, _) => 2,
            (c, AudioChannel::Left | AudioChannel::Right) => {
                return Err(eyre!(
                    "Audio with single output channel cannot have more than one channel ({c})."
                ));
            }
            (c, AudioChannel::Stereo) => c,
        };

        let mut samples = vec![];
        for s in self.0 {
            if channel == AudioChannel::Left {
                samples.push(0);
            }
            samples.push(s);
            if channel == AudioChannel::Right {
                samples.push(0);
            }
        }

        Ok(Self(
            SamplesBuffer::new(out_channels, sample_rate, samples).buffered(),
        ))
    }
}
