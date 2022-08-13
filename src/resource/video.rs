use crate::config::Config;
use crate::error;
use crate::resource::stream::Stream;
use crate::resource::FrameBuffer;
use std::path::Path;

pub fn video_from_file(path: &Path, _config: &Config) -> Result<(FrameBuffer, f64), error::Error> {
    Stream::new(path)?.pull_samples()
}
