use crate::resource::{
    AudioBackend, Color, Interpreter, LogFormat, StreamBackend, TimePrecision, Volume,
};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "defaults::verify_sha2")]
    #[serde(skip_serializing)]
    verify_sha2: Option<String>,
    #[serde(default = "defaults::blocks_per_row")]
    blocks_per_row: i32,
    #[serde(default = "defaults::volume")]
    volume: Volume,
    #[serde(default = "defaults::log_format")]
    log_format: LogFormat,
    #[serde(default = "defaults::time_precision")]
    time_precision: TimePrecision,
    #[serde(default = "defaults::interpreter")]
    interpreter: Interpreter,
    #[serde(default = "defaults::audio_backend")]
    audio_backend: AudioBackend,
    #[serde(default = "defaults::stream_backend")]
    stream_backend: StreamBackend,
    #[serde(default = "defaults::background")]
    background: Color,
}

mod defaults {
    use crate::resource::{
        AudioBackend, Color, Interpreter, LogFormat, StreamBackend, TimePrecision, Volume,
    };
    use cfg_if::cfg_if;

    #[inline(always)]
    pub fn verify_sha2() -> Option<String> {
        None
    }

    #[inline(always)]
    pub fn blocks_per_row() -> i32 {
        3
    }

    #[inline(always)]
    pub fn volume() -> Volume {
        Volume::Value(1.0)
    }

    #[inline(always)]
    pub fn log_format() -> LogFormat {
        LogFormat::RON
    }

    #[inline(always)]
    pub fn time_precision() -> TimePrecision {
        TimePrecision::RespectBoundaries
    }

    #[inline(always)]
    pub fn interpreter() -> Interpreter {
        Interpreter::Fasteval
    }

    #[inline(always)]
    pub fn audio_backend() -> AudioBackend {
        cfg_if! {
            if #[cfg(feature = "rodio")] {
                AudioBackend::Rodio
            } else {
                AudioBackend::None
            }
        }
    }

    #[inline(always)]
    pub fn stream_backend() -> StreamBackend {
        cfg_if! {
            if #[cfg(feature = "gstreamer")] {
                StreamBackend::Gst
            } else if #[cfg(feature = "ffmpeg")] {
                StreamBackend::Ffmpeg
            } else {
                StreamBackend::None
            }
        }
    }

    #[inline(always)]
    pub fn background() -> Color {
        Color::Transparent
    }
}

impl Config {
    pub fn init(&mut self) -> Result<()> {
        self.volume = self.volume.or(&defaults::volume());
        self.time_precision = self.time_precision.or(&defaults::time_precision());
        self.log_format = self.log_format.or(&defaults::log_format());
        self.interpreter = self.interpreter.or(&defaults::interpreter());
        self.audio_backend = self.audio_backend.or(&defaults::audio_backend());
        self.stream_backend = self.stream_backend.or(&defaults::stream_backend());
        self.background = self.background.or(&defaults::background());
        Ok(())
    }

    #[inline(always)]
    pub fn volume(&self) -> Volume {
        self.volume
    }

    pub fn verify_checksum(&self, task: String) -> Result<()> {
        if let Some(checksum) = self.verify_sha2.as_ref() {
            if checksum != &task {
                return Err(eyre!(
                    "Checksum of this task does not match the one on file.\n\
                    Current: {task}\n\
                    On file: {checksum}"
                ));
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub fn blocks_per_row(&self) -> i32 {
        self.blocks_per_row
    }

    #[inline(always)]
    pub fn log_format(&self) -> LogFormat {
        self.log_format
    }

    #[inline(always)]
    pub fn time_precision(&self) -> TimePrecision {
        self.time_precision
    }

    #[inline(always)]
    pub fn interpreter(&self) -> Interpreter {
        self.interpreter
    }

    #[inline(always)]
    pub fn audio_backend(&self) -> AudioBackend {
        self.audio_backend
    }

    #[inline(always)]
    pub fn stream_backend(&self) -> StreamBackend {
        self.stream_backend
    }

    #[inline(always)]
    pub fn background(&self) -> Color {
        self.background
    }
}

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OptionalConfig {
    #[serde(default)]
    volume: Volume,
    #[serde(default)]
    log_format: LogFormat,
    #[serde(default)]
    time_precision: TimePrecision,
    #[serde(default)]
    interpreter: Interpreter,
    #[serde(default)]
    audio_backend: AudioBackend,
    #[serde(default)]
    stream_backend: StreamBackend,
    #[serde(default)]
    background: Color,
}

impl OptionalConfig {
    pub fn fill_blanks(&self, base_config: &Config) -> Config {
        let mut config = base_config.clone();
        config.volume = self.volume.or(&base_config.volume);
        config.time_precision = self.time_precision.or(&config.time_precision);
        config.log_format = self.log_format.or(&base_config.log_format);
        config.interpreter = self.interpreter.or(&config.interpreter);
        config.audio_backend = self.audio_backend.or(&config.audio_backend);
        config.stream_backend = self.stream_backend.or(&config.stream_backend);
        config.background = self.background.or(&config.background);
        config
    }
}
