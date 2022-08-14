use crate::error;
use crate::error::Error::{ChecksumError, InvalidConfigError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    None,
    LastChannel,
    SeparateFile,
}

impl Default for TriggerType {
    #[inline(always)]
    fn default() -> Self {
        TriggerType::None
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    JSON,
    YAML,
}

impl Default for LogFormat {
    #[inline(always)]
    fn default() -> Self {
        LogFormat::JSON
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
pub enum LogCondition {
    None,
    Start,
    Stop,
    StartAndStop,
}

impl Default for LogCondition {
    #[inline(always)]
    fn default() -> Self {
        LogCondition::StartAndStop
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    trigger_type: TriggerType,
    #[serde(default = "defaults::use_trigger")]
    use_trigger: bool,
    #[serde(default = "defaults::verify_sha2")]
    #[serde(skip_serializing)]
    verify_sha2: Option<String>,
    #[serde(default = "defaults::blocks_per_row")]
    blocks_per_row: i32,
    #[serde(default = "defaults::nested_evolve")]
    nested_evolve: bool,
    #[serde(default)]
    base_volume: Option<f32>,
    #[serde(default)]
    fps_lock: f64,
    #[serde(default)]
    log_format: LogFormat,
    #[serde(default)]
    time_precision: TimePrecision,
    #[serde(default)]
    log_when: LogCondition,
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
    pub fn nested_evolve() -> bool {
        false
    }
}

impl Config {
    #[inline(always)]
    pub fn set_trigger(&mut self, state: bool) {
        self.use_trigger = state;
    }

    #[inline(always)]
    pub fn trigger_type(&self) -> &TriggerType {
        &self.trigger_type
    }

    #[inline(always)]
    pub fn use_trigger(&self) -> bool {
        if matches!(self.trigger_type, TriggerType::None) {
            false
        } else {
            self.use_trigger
        }
    }

    #[inline(always)]
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
    pub fn fps_lock(&self) -> f64 {
        self.fps_lock
    }

    #[inline(always)]
    pub fn nested_evolve(&self) -> bool {
        self.nested_evolve
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
    pub fn log_when(&self) -> LogCondition {
        self.log_when
    }

    #[inline(always)]
    pub fn verify(&self) -> Result<(), error::Error> {
        if matches!(self.trigger_type, TriggerType::None) && self.use_trigger {
            Err(InvalidConfigError(
                "Configuration `use_trigger = true` is not consistent with `trigger_type = none`"
                    .to_owned(),
            ))
        } else {
            Ok(())
        }
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
    trigger_type: Option<TriggerType>,
    #[serde(default)]
    use_trigger: Option<bool>,
    #[serde(default)]
    nested_evolve: Option<bool>,
    #[serde(default)]
    base_volume: Option<Option<f32>>,
    #[serde(default)]
    fps_lock: Option<f64>,
    #[serde(default)]
    log_format: Option<LogFormat>,
    #[serde(default)]
    time_precision: Option<TimePrecision>,
    #[serde(default)]
    log_when: Option<LogCondition>,
}

impl OptionalConfig {
    pub fn fill_blanks(&self, base_config: &Config) -> Config {
        let mut config = base_config.clone();
        if let Some(v) = self.trigger_type {
            config.trigger_type = v;
        }
        if let Some(v) = self.use_trigger {
            config.use_trigger = v;
        }
        if let Some(v) = self.nested_evolve {
            config.nested_evolve = v;
        }
        if let Some(v) = self.base_volume {
            config.base_volume = v;
        }
        if let Some(v) = self.fps_lock {
            config.fps_lock = v;
        }
        if let Some(v) = self.log_format {
            config.log_format = v;
        }
        if let Some(v) = self.time_precision {
            config.time_precision = v;
        }
        if let Some(v) = self.log_when {
            config.log_when = v;
        }
        config
    }
}
