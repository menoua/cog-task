use crate::resource::{color::Color, Interpreter, LogFormat, MediaBackend, TimePrecision};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "defaults::use_trigger")]
    use_trigger: bool,
    #[serde(default = "defaults::verify_sha2")]
    #[serde(skip_serializing)]
    verify_sha2: Option<String>,
    #[serde(default = "defaults::blocks_per_row")]
    blocks_per_row: i32,
    #[serde(default = "defaults::volume")]
    base_volume: f32,
    #[serde(default)]
    log_format: LogFormat,
    #[serde(default)]
    time_precision: TimePrecision,
    #[serde(default)]
    math_interpreter: Interpreter,
    #[serde(default)]
    media_backend: MediaBackend,
    #[serde(default)]
    background: Color,
}

mod defaults {
    #[inline(always)]
    pub fn use_trigger() -> bool {
        true
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
    pub fn volume() -> f32 {
        1.0
    }
}

impl Config {
    #[inline(always)]
    pub fn use_trigger(&self) -> bool {
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
    pub fn base_volume(&self) -> f32 {
        self.base_volume
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
    pub fn math_interpreter(&self) -> Interpreter {
        self.math_interpreter
    }

    #[inline(always)]
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
    use_trigger: Option<bool>,
    #[serde(default)]
    base_volume: Option<f32>,
    #[serde(default)]
    log_format: LogFormat,
    #[serde(default)]
    time_precision: TimePrecision,
    #[serde(default)]
    math_interpreter: Interpreter,
    #[serde(default)]
    media_backend: MediaBackend,
    #[serde(default)]
    background: Option<Color>,
}

impl OptionalConfig {
    pub fn fill_blanks(&self, base_config: &Config) -> Config {
        let mut config = base_config.clone();
        if let Some(v) = self.use_trigger {
            config.use_trigger = v;
        }
        if let Some(v) = self.base_volume {
            config.base_volume = v;
        }
        config.log_format = self.log_format.or(&base_config.log_format);
        config.time_precision = self.time_precision.or(&config.time_precision);
        config.math_interpreter = self.math_interpreter.or(&config.math_interpreter);
        config.media_backend = self.media_backend.or(&config.media_backend);
        if let Some(v) = self.background {
            config.background = v;
        }
        config
    }
}
