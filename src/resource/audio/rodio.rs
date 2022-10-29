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
            File::open(&path).wrap_err_with(|| format!("Failed to open audio file: {path:?}"))?,
        ))
        .wrap_err_with(|| format!("Failed to decode audio file: {path:?}"))?;

        let sample_rate = decoder.sample_rate();
        let in_channels = decoder.channels() as i16;
        let out_channels = in_channels;

        let mut c = -1;
        let mut samples = vec![];
        for s in decoder {
            c = (c + 1) % in_channels;
            samples.push(s);
        }

        Ok(Self(
            SamplesBuffer::new(out_channels as u16, sample_rate, samples).buffered(),
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

    pub fn interlace(self, mut other: Self) -> Result<Self> {
        let sample_rate = self.0.sample_rate();
        let in_channels = self.0.channels() as i16;
        let out_channels = in_channels + 1;

        if other.0.sample_rate() != sample_rate {
            return Err(eyre!(
                "Trigger (?) has different sampling rate than corresponding audio"
            ));
        }
        if other.0.channels() != 1 {
            return Err(eyre!("Trigger (?) should have exactly 1 channel"));
        }

        let mut c = -1;
        let mut samples = vec![];
        for s in self.0 {
            c = (c + 1) % in_channels;
            samples.push(s);
            if c == in_channels - 1 {
                if let Some(s) = other.0.next() {
                    samples.push(s);
                }
            }
        }
        if other.0.next().is_some() {
            return Err(eyre!("Trigger for (?) is longer than itself."));
        }

        Ok(Self(
            SamplesBuffer::new(out_channels as u16, sample_rate, samples).buffered(),
        ))
    }

    pub fn drop_last(self) -> Result<Self> {
        let sample_rate = self.0.sample_rate();
        let in_channels = self.0.channels() as i16;
        let out_channels = in_channels - 1;
        if out_channels == 0 {
            return Err(eyre!(
                "Audio with internal trigger should have at least one channel."
            ));
        }

        let mut c = -1;
        let mut samples = vec![];
        for s in self.0 {
            c = (c + 1) % in_channels;
            if c < in_channels - 1 {
                samples.push(s);
            }
        }

        Ok(Self(
            SamplesBuffer::new(out_channels as u16, sample_rate, samples).buffered(),
        ))
    }
}
