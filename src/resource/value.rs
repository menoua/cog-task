use crate::resource::{AudioBuffer, FrameBuffer, Stream};
use eframe::egui::{TextureId, Vec2};
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub enum ResourceValue {
    Ref(PathBuf),
    Text(Arc<String>),
    Image(TextureId, Vec2),
    Audio(AudioBuffer),
    Video(FrameBuffer, f64),
    Stream(Stream),
}

impl Debug for ResourceValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceValue::Ref(path) => {
                write!(f, "{path:?}")
            }
            ResourceValue::Text(_) => {
                write!(f, "[Text]")
            }
            ResourceValue::Image(_, size) => {
                write!(f, "[Image ({} x {})]", size.x, size.y)
            }
            ResourceValue::Audio(buffer) => {
                write!(
                    f,
                    "[Audio ({:?} @ {}Hz)]",
                    buffer.duration(),
                    buffer.sample_rate()
                )
            }
            ResourceValue::Video(frames, fps) => {
                write!(f, "[Cached video ({} frames @ {}fps)]", frames.len(), fps,)
            }
            ResourceValue::Stream(stream) => {
                write!(
                    f,
                    "[Buffered stream ({:?} @ {}fps)]",
                    stream.duration(),
                    stream.framerate()
                )
            }
        }
    }
}
