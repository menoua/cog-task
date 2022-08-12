use crate::config::{Config, TriggerType};
use crate::error;
use crate::error::Error::{AudioDecodingError, ResourceLoadError, TriggerConfigError};
use rodio::buffer::SamplesBuffer;
use rodio::source::Buffered;
use rodio::{Decoder, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub fn audio_from_file(
    path: &Path,
    config: &Config,
) -> Result<Buffered<SamplesBuffer<i16>>, error::Error> {
    let gain = config.base_gain();
    let trigger_type = config.trigger_type();
    let use_trigger = config.use_trigger();

    let decoder = Decoder::new(BufReader::new(File::open(&path).map_err(|e| {
        ResourceLoadError(format!("Failed to open audio file: `{path:?}`\n{e:#?}"))
    })?))
    .map_err(|e| AudioDecodingError(format!("Failed to decode audio file: `{path:?}`\n{e:#?}")))?;

    let sample_rate = decoder.sample_rate();
    let in_channels = decoder.channels() as i16;
    let out_channels = match (trigger_type, use_trigger) {
        (TriggerType::None, _) => in_channels,
        (TriggerType::LastChannel, false) => in_channels - 1,
        (TriggerType::LastChannel, true) => in_channels,
        (TriggerType::SeparateFile, false) => in_channels,
        (TriggerType::SeparateFile, true) => in_channels + 1,
    };
    let mut trigger = if matches!(trigger_type, TriggerType::SeparateFile) && use_trigger {
        let ext = if let Some(ext) = path.extension() {
            Ok(ext.to_str().unwrap().to_owned())
        } else {
            Err(ResourceLoadError(format!(
                "Audio file name {path:?} should have extension"
            )))
        }?;
        let path = path.with_extension(format!("trig.{ext}"));
        let decoder = Decoder::new(BufReader::new(File::open(&path).map_err(|e| {
            ResourceLoadError(format!(
                "Failed to open audio trigger file: `{path:?}`:\n{e:#?}"
            ))
        })?))
        .map_err(|e| {
            AudioDecodingError(format!(
                "Failed to decode audio trigger file: `{path:?}`:\n{e:#?}"
            ))
        })?;
        if decoder.sample_rate() != sample_rate {
            return Err(TriggerConfigError(format!(
                "Trigger ({path:?}) has different sampling rate than corresponding audio"
            )));
        }
        if decoder.channels() != 1 {
            return Err(TriggerConfigError(format!(
                "Trigger ({path:?}) should have exactly 1 channel"
            )));
        }
        Some(decoder)
    } else {
        None
    };

    let mut c = -1;
    let mut samples = vec![];
    for s in decoder {
        c = (c + 1) % in_channels;
        if c < in_channels - 1 || trigger.is_none() || use_trigger {
            if let Some(gain) = gain {
                samples.push((gain * s as f32) as i16);
            } else {
                samples.push(s);
            }
        }
        if c == in_channels - 1 {
            if let Some(trigger) = &mut trigger {
                if let Some(s) = trigger.next() {
                    samples.push(s);
                } else {
                    return Err(TriggerConfigError(format!(
                        "Trigger for ({path:?}) is shorter than itself"
                    )));
                }
            }
        }
    }
    if let Some(mut trigger) = trigger {
        if trigger.next().is_some() {
            return Err(TriggerConfigError(format!(
                "Trigger for ({path:?}) is longer than itself"
            )));
        }
    }

    Ok(SamplesBuffer::new(out_channels as u16, sample_rate, samples).buffered())
}
