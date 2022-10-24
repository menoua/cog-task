pub mod audio;
pub mod color;
pub mod image;
pub mod key;
pub mod stream;
pub mod text;
pub mod video;

use crate::resource::image::{image_from_file, svg_from_bytes, svg_from_file};
use audio::audio_from_file;
use text::text_or_file;
use video::video_from_file;

use crate::assets::{IMAGE_FIXATION, IMAGE_RUSTACEAN};
use crate::config::Config;
use crate::env::Env;
use crate::resource::stream::{stream_from_file, Stream};
use eframe::egui::mutex::RwLock;
use eframe::egui::{TextureId, Vec2};
use eframe::epaint;
use eyre::{eyre, Result};
use rodio::buffer::SamplesBuffer;
use rodio::source::Buffered;
use rodio::Source;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

pub type FrameBuffer = Arc<Vec<(TextureId, Vec2)>>;
pub type AudioBuffer = Buffered<SamplesBuffer<i16>>;

#[derive(Clone)]
pub enum ResourceValue {
    Text(Arc<String>),
    Image(TextureId, Vec2),
    Audio(AudioBuffer),
    Video(FrameBuffer, f64),
    Stream(Stream),
}

impl Debug for ResourceValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
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
                    buffer.total_duration().unwrap(),
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

#[derive(Default, Debug, Clone)]
pub struct ResourceMap(Arc<Mutex<HashMap<PathBuf, ResourceValue>>>);

impl ResourceMap {
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.0.lock().unwrap().clear();
    }

    pub fn preload_block(
        &mut self,
        resources: Vec<PathBuf>,
        tex_manager: Arc<RwLock<epaint::TextureManager>>,
        config: &Config,
        env: &Env,
    ) -> Result<()> {
        // Lock map
        let mut map = self.0.lock().unwrap();

        // Clean up existing resource map
        map.clear();

        // Load default fixation image
        let src = PathBuf::from_str("fixation.svg").unwrap();
        map.entry(src.clone()).or_insert({
            let tex_manager = tex_manager.clone();
            let (texture, size) = svg_from_bytes(tex_manager, IMAGE_FIXATION, &src)?;
            let data = ResourceValue::Image(texture, size);
            println!("+ default fixation : {data:?}");
            data
        });
        let mut default_fixation = true;

        // Load default rustacean image
        let src = PathBuf::from_str("rustacean.svg").unwrap();
        map.entry(src.clone()).or_insert({
            let tex_manager = tex_manager.clone();
            let (texture, size) = svg_from_bytes(tex_manager, IMAGE_RUSTACEAN, &src)?;
            let data = ResourceValue::Image(texture, size);
            println!("+ default rustacean : {data:?}");
            data
        });
        let mut default_rustacean = true;

        // Load resources used in new block
        for src in resources {
            let mut is_new = !map.contains_key(&src);
            match src.to_str().unwrap() {
                "fixation.svg" => {
                    if default_fixation {
                        is_new = true;
                        default_fixation = false;
                    }
                }
                "rustacean.svg" => {
                    if default_rustacean {
                        is_new = true;
                        default_rustacean = false;
                    }
                }
                _ => {}
            }

            if is_new {
                let path = env.resource().join(&src);
                let extn = path
                    .extension()
                    .expect("Data file names need to have extensions")
                    .to_str()
                    .unwrap();
                let (extn, mode) = match extn.split_once('#') {
                    None => (extn, ""),
                    Some(pair) => pair,
                };
                let path = path.with_extension(extn);
                let extn = if mode.is_empty() { extn } else { mode };
                let data = match extn {
                    "txt" | "ron" => {
                        let text = std::fs::read_to_string(&path)?;
                        Ok(ResourceValue::Text(Arc::new(text)))
                    }
                    "png" | "jpg" | "jpeg" | "bmp" | "tiff" | "ico" => {
                        let tex_manager = tex_manager.clone();
                        let (texture, size) = image_from_file(tex_manager, &path)?;
                        Ok(ResourceValue::Image(texture, size))
                    }
                    "svg" => {
                        let tex_manager = tex_manager.clone();
                        let (texture, size) = svg_from_file(tex_manager, &path)?;
                        Ok(ResourceValue::Image(texture, size))
                    }
                    "wav" | "flac" | "ogg" => {
                        Ok(ResourceValue::Audio(audio_from_file(&path, config)?))
                    }
                    "avi" | "gif" | "mkv" | "mov" | "mp4" | "mpg" | "webm" => {
                        let tex_manager = tex_manager.clone();
                        let (frames, framerate) = video_from_file(tex_manager, &path, config)?;
                        Ok(ResourceValue::Video(frames, framerate))
                    }
                    "stream" => {
                        let tex_manager = tex_manager.clone();
                        Ok(ResourceValue::Stream(stream_from_file(
                            tex_manager,
                            &path,
                            config,
                        )?))
                    }
                    _ => Err(eyre!(
                        "Invalid extension `{extn}` with mode `{mode}` for data file {src:?}"
                    )),
                }?;
                println!("+ {src:?} : {data:?}");
                map.insert(src, data);
            }
        }

        Ok(())
    }

    pub fn fetch(&self, src: &PathBuf) -> Result<ResourceValue> {
        if let Some(res) = self.0.lock().unwrap().get(src) {
            Ok(res.clone())
        } else {
            Err(eyre!("Tried to fetch unexpected resource: {src:?}"))
        }
    }

    pub fn fetch_text(&self, text: &str) -> Result<String> {
        let text: String = match text_or_file(text) {
            Some(src) => {
                if let ResourceValue::Text(text) = self.fetch(&src)? {
                    Ok((*text).clone())
                } else {
                    Err(eyre!("Tried to read non-text file as text: {src:?}"))
                }
            }
            None => Ok(text.to_owned()),
        }?;
        Ok(text)
    }
}
