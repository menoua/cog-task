#[cfg(feature = "stream")]
use crate::resource::MediaBackend;
use crate::resource::{Color, Interpreter, LogFormat};
#[cfg(feature = "audio")]
use crate::resource::{TimePrecision, UseTrigger, Volume};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[cfg(feature = "audio")]
    #[serde(default = "defaults::use_trigger")]
    use_trigger: UseTrigger,
    #[serde(default = "defaults::verify_sha2")]
    #[serde(skip_serializing)]
    verify_sha2: Option<String>,
    #[serde(default = "defaults::blocks_per_row")]
    blocks_per_row: i32,
    #[cfg(feature = "audio")]
    #[serde(default = "defaults::volume")]
    volume: Volume,
    #[serde(default = "defaults::log_format")]
    log_format: LogFormat,
    #[cfg(feature = "audio")]
    #[serde(default = "defaults::time_precision")]
    time_precision: TimePrecision,
    #[serde(default = "defaults::math_interpreter")]
    math_interpreter: Interpreter,
    #[cfg(feature = "stream")]
    #[serde(default = "defaults::media_backend")]
    media_backend: MediaBackend,
    #[serde(default = "defaults::background")]
    background: Color,
}

mod defaults {
    #[cfg(feature = "stream")]
    use crate::resource::MediaBackend;
    use crate::resource::{Color, Interpreter, LogFormat};
    #[cfg(feature = "audio")]
    use crate::resource::{TimePrecision, UseTrigger, Volume};

    #[inline(always)]
    #[cfg(feature = "audio")]
    pub fn use_trigger() -> UseTrigger {
        UseTrigger::Yes
    }

    #[inline(always)]
    pub fn verify_sha2() -> Option<String> {
        None
    }

    #[inline(always)]
    pub fn blocks_per_row() -> i32 {
        3
    }

    #[inline(always)]
    #[cfg(feature = "audio")]
    pub fn volume() -> Volume {
        Volume::Value(1.0)
    }

    #[inline(always)]
    pub fn log_format() -> LogFormat {
        LogFormat::RON
    }

    #[inline(always)]
    #[cfg(feature = "audio")]
    pub fn time_precision() -> TimePrecision {
        TimePrecision::RespectBoundaries
    }

    #[inline(always)]
    pub fn math_interpreter() -> Interpreter {
        Interpreter::Fasteval
    }

    #[inline(always)]
    #[cfg(feature = "stream")]
    pub fn media_backend() -> MediaBackend {
        MediaBackend::None
    }

    #[inline(always)]
    pub fn background() -> Color {
        Color::Transparent
    }
}

impl Config {
    pub fn init(&mut self) -> Result<()> {
        #[cfg(feature = "audio")]
        {
            self.volume = self.volume.or(&defaults::volume());
            self.use_trigger = self.use_trigger.or(&defaults::use_trigger());
            self.time_precision = self.time_precision.or(&defaults::time_precision());
        }
        self.log_format = self.log_format.or(&defaults::log_format());
        self.math_interpreter = self.math_interpreter.or(&defaults::math_interpreter());
        #[cfg(feature = "stream")]
        {
            self.media_backend = self.media_backend.or(&defaults::media_backend());
        }
        self.background = self.background.or(&defaults::background());
        Ok(())
    }

    #[inline(always)]
    #[cfg(feature = "audio")]
    pub fn volume(&self) -> Volume {
        self.volume
    }

    #[inline(always)]
    #[cfg(feature = "audio")]
    pub fn use_trigger(&self) -> UseTrigger {
        self.use_trigger
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
    #[cfg(feature = "audio")]
    pub fn time_precision(&self) -> TimePrecision {
        self.time_precision
    }

    #[inline(always)]
    pub fn math_interpreter(&self) -> Interpreter {
        self.math_interpreter
    }

    #[inline(always)]
    #[cfg(feature = "stream")]
    pub fn media_backend(&self) -> MediaBackend {
        self.media_backend
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
    #[cfg(feature = "audio")]
    volume: Volume,
    #[serde(default)]
    #[cfg(feature = "audio")]
    use_trigger: UseTrigger,
    #[serde(default)]
    log_format: LogFormat,
    #[serde(default)]
    #[cfg(feature = "audio")]
    time_precision: TimePrecision,
    #[serde(default)]
    math_interpreter: Interpreter,
    #[serde(default)]
    #[cfg(feature = "stream")]
    media_backend: MediaBackend,
    #[serde(default)]
    background: Color,
}

impl OptionalConfig {
    pub fn fill_blanks(&self, base_config: &Config) -> Config {
        let mut config = base_config.clone();
        #[cfg(feature = "audio")]
        {
            config.volume = self.volume.or(&base_config.volume);
            config.use_trigger = self.use_trigger.or(&base_config.use_trigger);
            config.time_precision = self.time_precision.or(&config.time_precision);
        }
        config.log_format = self.log_format.or(&base_config.log_format);
        config.math_interpreter = self.math_interpreter.or(&config.math_interpreter);
        #[cfg(feature = "stream")]
        {
            config.media_backend = self.media_backend.or(&config.media_backend);
        }
        config.background = self.background.or(&config.background);
        config
    }
}
