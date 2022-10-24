use eyre::{Context, Result};
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
    pub fn new() -> Result<Self> {
        let (_audio_stream, audio_stream_handle) =
            OutputStream::try_default().wrap_err("Failed to obtain audio output stream.")?;

        Ok(Self {
            _audio_stream,
            audio_stream_handle,
        })
    }

    #[inline]
    pub fn audio(&self) -> Result<Sink> {
        Sink::try_new(&self.audio_stream_handle).wrap_err("Failed to create audio sink.")
    }
}
