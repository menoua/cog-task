use crate::error;
use crate::error::Error::ChecksumError;
use crate::resource::color::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    JSON,
    YAML,
    RON,
}

impl Default for LogFormat {
    #[inline(always)]
    fn default() -> Self {
        LogFormat::RON
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimePrecision {
    RespectIntervals,
    RespectBoundaries,
}

impl Default for TimePrecision {
    #[inline(always)]
    fn default() -> Self {
        TimePrecision::RespectBoundaries
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaBackend {
    None,
    Gst,
    Ffmpeg,
}

impl Default for MediaBackend {
    #[inline(always)]
    fn default() -> Self {
        MediaBackend::None
    }
}

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
    #[serde(default)]
    base_volume: Option<f32>,
    #[serde(default)]
    log_format: LogFormat,
    #[serde(default)]
    time_precision: TimePrecision,
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
}

impl Config {
    #[inline(always)]
    pub fn use_trigger(&self) -> bool {
        self.use_trigger
    }

    pub fn verify_checksum(&self, task: String) -> Result<(), error::Error> {
        if let Some(checksum) = self.verify_sha2.as_ref() {
            if checksum != &task {
                return Err(ChecksumError(format!(
                    "Checksum of this task does not match the one on file.\n\
                    Current: {task}\n\
                    On file: {checksum}"
                )));
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub fn blocks_per_row(&self) -> i32 {
        self.blocks_per_row
    }

    #[inline(always)]
    pub fn base_volume(&self) -> Option<f32> {
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
    pub fn media_backend(&self) -> MediaBackend {
        self.media_backend
    }

    #[inline(always)]
    pub fn background(&self) -> Color {
        self.background
    }

    pub fn volume(&self, vol: Option<f32>) -> Option<f32> {
        match (self.base_volume, vol) {
            (Some(v), Some(w)) => Some(v * w),
            (Some(v), None) => Some(v),
            (None, Some(w)) => Some(w),
            (None, None) => None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OptionalConfig {
    #[serde(default)]
    use_trigger: Option<bool>,
    #[serde(default)]
    base_volume: Option<Option<f32>>,
    #[serde(default)]
    log_format: Option<LogFormat>,
    #[serde(default)]
    time_precision: Option<TimePrecision>,
    #[serde(default)]
    media_backend: Option<MediaBackend>,
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
        if let Some(v) = self.log_format {
            config.log_format = v;
        }
        if let Some(v) = self.time_precision {
            config.time_precision = v;
        }
        if let Some(v) = self.media_backend {
            config.media_backend = v;
        }
        if let Some(v) = self.background {
            config.background = v;
        }
        config
    }
}
