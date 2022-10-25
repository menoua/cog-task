#[cfg(feature = "stream")]
use crate::resource::stream::Stream;
use eframe::egui::{TextureId, Vec2};
#[cfg(feature = "audio")]
use rodio::{buffer::SamplesBuffer, source::Buffered, Source};
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "stream")]
pub type FrameBuffer = Arc<Vec<(TextureId, Vec2)>>;
#[cfg(feature = "audio")]
pub type AudioBuffer = Buffered<SamplesBuffer<i16>>;

#[derive(Clone)]
pub enum ResourceValue {
    Ref(PathBuf),
    Text(Arc<String>),
    Image(TextureId, Vec2),
    #[cfg(feature = "audio")]
    Audio(AudioBuffer),
    #[cfg(feature = "stream")]
    Video(FrameBuffer, f64),
    #[cfg(feature = "stream")]
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
            #[cfg(feature = "audio")]
            ResourceValue::Audio(buffer) => {
                write!(
                    f,
                    "[Audio ({:?} @ {}Hz)]",
                    buffer.total_duration().unwrap(),
                    buffer.sample_rate()
                )
            }
            #[cfg(feature = "stream")]
            ResourceValue::Video(frames, fps) => {
                write!(f, "[Cached video ({} frames @ {}fps)]", frames.len(), fps,)
            }
            #[cfg(feature = "stream")]
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
