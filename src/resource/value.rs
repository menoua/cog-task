use crate::resource::{AudioBuffer, FrameBuffer, Mask2D, Stream, Texture, VideoBuffer};
use eframe::egui::{TextureId, Vec2};
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub enum ResourceValue {
    Ref(PathBuf),
    Text(Arc<String>),
    Image(TextureId, Vec2),
    Mask(Mask2D),
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
            ResourceValue::Mask(mask) => {
                write!(f, "[Mask ({} x {})]", mask.size().x, mask.size().y)
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

impl From<PathBuf> for ResourceValue {
    fn from(value: PathBuf) -> Self {
        Self::Ref(value)
    }
}

impl From<String> for ResourceValue {
    fn from(value: String) -> Self {
        Self::Text(Arc::new(value))
    }
}

impl From<Texture> for ResourceValue {
    fn from(value: Texture) -> Self {
        Self::Image(value.0, value.1)
    }
}

impl From<Mask2D> for ResourceValue {
    fn from(value: Mask2D) -> Self {
        Self::Mask(value)
    }
}

impl From<AudioBuffer> for ResourceValue {
    fn from(value: AudioBuffer) -> Self {
        Self::Audio(value)
    }
}

impl From<VideoBuffer> for ResourceValue {
    fn from(value: VideoBuffer) -> Self {
        Self::Video(value.0, value.1)
    }
}

impl From<Stream> for ResourceValue {
    fn from(value: Stream) -> Self {
        Self::Stream(value)
    }
}
