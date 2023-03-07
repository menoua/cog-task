use crate::server::Config;
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::path::Path;
use std::time::Duration;

#[cfg(feature = "rodio")]
mod rodio;

#[derive(Debug, Copy, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AudioChannel {
    Stereo,
    Left,
    Right,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioBackend {
    None,
    Inherit,
    #[cfg(feature = "rodio")]
    Rodio,
}

#[derive(Clone)]
pub enum AudioBuffer {
    None,
    #[cfg(feature = "rodio")]
    Rodio(rodio::Buffer),
}

pub enum AudioSink {
    None,
    #[cfg(feature = "rodio")]
    Rodio(rodio::Sink),
}

pub enum AudioDevice {
    None,
    #[cfg(feature = "rodio")]
    Rodio(rodio::Device),
}

impl AudioDevice {
    pub fn try_clone(&self) -> Result<Self> {
        match self {
            AudioDevice::None => Ok(AudioDevice::None),
            #[cfg(feature = "rodio")]
            AudioDevice::Rodio(_) => rodio::Device::new().map(AudioDevice::Rodio),
        }
    }
}

impl Default for AudioChannel {
    fn default() -> Self {
        AudioChannel::Stereo
    }
}

impl Default for AudioBackend {
    #[inline(always)]
    fn default() -> Self {
        AudioBackend::Inherit
    }
}

impl AudioBackend {
    pub fn or(&self, other: &Self) -> Self {
        if let Self::Inherit = self {
            *other
        } else {
            *self
        }
    }
}

#[derive(Copy, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Volume {
    Inherit,
    Value(f32),
}

impl Default for Volume {
    #[inline(always)]
    fn default() -> Self {
        Volume::Inherit
    }
}

impl Debug for Volume {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Volume::Value(vol) = self {
            write!(f, "{vol}")
        } else {
            write!(f, "inherit")
        }
    }
}

impl Volume {
    pub fn and(&self, other: &Self) -> Self {
        match (self, other) {
            (&Self::Value(x), &Self::Value(y)) => Self::Value(x * y),
            (Self::Value(_), _) => *self,
            (Self::Inherit, _) => *other,
        }
    }

    pub fn or(&self, other: &Self) -> Self {
        if let Self::Inherit = self {
            *other
        } else {
            *self
        }
    }

    pub fn value(&self) -> f32 {
        if let &Self::Value(x) = self {
            x
        } else {
            1.0
        }
    }
}

impl From<f32> for Volume {
    fn from(v: f32) -> Self {
        Self::Value(v.max(0.0))
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimePrecision {
    Inherit,
    RespectIntervals,
    RespectBoundaries,
}

impl Default for TimePrecision {
    #[inline(always)]
    fn default() -> Self {
        TimePrecision::Inherit
    }
}

impl TimePrecision {
    pub fn or(&self, other: &Self) -> Self {
        if let Self::Inherit = self {
            *other
        } else {
            *self
        }
    }
}

#[allow(unused_variables)]
pub fn audio_from_file(path: &Path, config: &Config) -> Result<AudioBuffer> {
    match config.audio_backend() {
        AudioBackend::None => Err(eyre!("Cannot load audio file with backend=None.")),
        AudioBackend::Inherit => Err(eyre!("Cannot load audio file with backend=Inherit.")),
        #[cfg(feature = "rodio")]
        AudioBackend::Rodio => rodio::Buffer::new(path, config).map(AudioBuffer::Rodio),
    }
}

impl AudioBuffer {
    pub fn duration(&self) -> Duration {
        match self {
            AudioBuffer::None => Duration::default(),
            #[cfg(feature = "rodio")]
            AudioBuffer::Rodio(x) => x.duration(),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        match self {
            AudioBuffer::None => 0,
            #[cfg(feature = "rodio")]
            AudioBuffer::Rodio(x) => x.sample_rate(),
        }
    }

    pub fn channels(&self) -> u16 {
        match self {
            AudioBuffer::None => 0,
            #[cfg(feature = "rodio")]
            AudioBuffer::Rodio(x) => x.channels(),
        }
    }

    pub fn interlaced(self, other: AudioBuffer) -> Result<AudioBuffer> {
        match (self, other) {
            #[cfg(feature = "rodio")]
            (AudioBuffer::Rodio(x), AudioBuffer::Rodio(y)) => {
                x.interlaced(y).map(AudioBuffer::Rodio)
            }
            (_, _) => Err(eyre!("Cannot interlace audio buffers of different types.")),
        }
    }

    pub fn with_direction(self, channel: AudioChannel) -> Result<AudioBuffer> {
        if channel == AudioChannel::Stereo {
            return Ok(self);
        }

        if self.channels() > 1 {
            return Err(eyre!(
                "Setting directional audio output only supported for mono audio buffer."
            ));
        }

        match self {
            #[cfg(feature = "rodio")]
            AudioBuffer::Rodio(x) => x.with_direction(channel).map(AudioBuffer::Rodio),
            _ => Err(eyre!(
                "Cannot set channel for audio buffer with backend=None."
            )),
        }
    }
}

impl AudioSink {
    pub fn pause(&mut self) -> Result<()> {
        match self {
            AudioSink::None => Err(eyre!("Cannot pause audio sink with backend=None.")),
            #[cfg(feature = "rodio")]
            AudioSink::Rodio(sink) => {
                sink.pause();
                Ok(())
            }
        }
    }

    #[allow(unused_variables)]
    pub fn set_volume(&mut self, volume: f32) -> Result<()> {
        match self {
            AudioSink::None => Err(eyre!("Cannot pause audio sink with backend=None.")),
            #[cfg(feature = "rodio")]
            AudioSink::Rodio(sink) => {
                sink.set_volume(volume);
                Ok(())
            }
        }
    }

    pub fn queue(&mut self, buffer: AudioBuffer) -> Result<()> {
        match (self, buffer) {
            (AudioSink::None, _) => Err(eyre!("Cannot queue audio on sink=None.")),
            #[cfg(feature = "rodio")]
            (AudioSink::Rodio(sink), AudioBuffer::Rodio(buffer)) => {
                sink.queue(buffer);
                Ok(())
            }
            #[allow(unreachable_patterns)]
            (_, _) => Err(eyre!("Cannot queue audio on incompatible sink.")),
        }
    }

    pub fn repeat(&mut self, buffer: AudioBuffer) -> Result<()> {
        match (self, buffer) {
            (AudioSink::None, _) => Err(eyre!("Cannot repeat audio on sink=None.")),
            #[cfg(feature = "rodio")]
            (AudioSink::Rodio(sink), AudioBuffer::Rodio(buffer)) => {
                sink.repeat(buffer);
                Ok(())
            }
            #[allow(unreachable_patterns)]
            (_, _) => Err(eyre!("Cannot repeat audio on incompatible sink.")),
        }
    }

    pub fn play(&mut self) -> Result<()> {
        match self {
            AudioSink::None => Err(eyre!("Cannot play audio sink with backend=None.")),
            #[cfg(feature = "rodio")]
            AudioSink::Rodio(sink) => {
                sink.play();
                Ok(())
            }
        }
    }

    pub fn stop(&mut self) -> Result<()> {
        match self {
            AudioSink::None => Err(eyre!("Cannot stop audio sink with backend=None.")),
            #[cfg(feature = "rodio")]
            AudioSink::Rodio(sink) => {
                sink.stop();
                Ok(())
            }
        }
    }

    pub fn empty(&self) -> Result<bool> {
        match self {
            AudioSink::None => Ok(true),
            #[cfg(feature = "rodio")]
            AudioSink::Rodio(sink) => Ok(sink.empty()),
        }
    }

    pub fn detach(self) -> Result<()> {
        match self {
            AudioSink::None => Ok(()),
            #[cfg(feature = "rodio")]
            AudioSink::Rodio(sink) => {
                sink.detach();
                Ok(())
            }
        }
    }
}

impl AudioDevice {
    pub fn new(config: &Config) -> Result<Self> {
        match config.audio_backend() {
            AudioBackend::None => Err(eyre!("Cannot obtain audio device with backend=None.")),
            AudioBackend::Inherit => Err(eyre!("Cannot obtain audio device with backend=None.")),
            #[cfg(feature = "rodio")]
            AudioBackend::Rodio => rodio::Device::new().map(Self::Rodio),
        }
    }

    pub fn sink(&self) -> Result<AudioSink> {
        match self {
            AudioDevice::None => Err(eyre!("Cannot create audio sink with backend=None.")),
            #[cfg(feature = "rodio")]
            AudioDevice::Rodio(device) => device.sink().map(AudioSink::Rodio),
        }
    }
}
