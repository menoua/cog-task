use crate::error;
use crate::error::Error::IoAccessError;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::fmt::{Debug, Formatter};

pub struct IO {
    _audio_stream: OutputStream,
    audio_stream_handle: OutputStreamHandle,
}

impl Debug for IO {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "<IO>")
    }
}

impl IO {
    pub fn new() -> Result<Self, error::Error> {
        let (_audio_stream, audio_stream_handle) = OutputStream::try_default()
            .map_err(|e| IoAccessError(format!("Failed to obtain audio output stream:\n{e:#?}")))?;

        Ok(Self {
            _audio_stream,
            audio_stream_handle,
        })
    }

    #[inline(always)]
    pub fn audio(&self) -> Result<Sink, error::Error> {
        Sink::try_new(&self.audio_stream_handle)
            .map_err(|e| IoAccessError(format!("Failed to create audio sink:\n{e:#?}")))
    }
}
