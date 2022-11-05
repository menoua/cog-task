pub mod address;
pub mod audio;
pub mod color;
pub mod function;
pub mod image;
pub mod key;
pub mod logger;
pub mod stream;
pub mod text;
pub mod trigger;
pub mod value;

pub use crate::resource::image::*;
pub use address::*;
pub use audio::*;
pub use color::*;
pub use function::*;
pub use key::*;
pub use logger::*;
pub use stream::*;
pub use text::*;
pub use trigger::Trigger;
pub use value::*;

use crate::assets::{IMAGE_FIXATION, IMAGE_RUSTACEAN};
use crate::server::{Config, Env};
use eframe::egui::mutex::RwLock;
use eframe::epaint;
use eyre::{eyre, Context, Result};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ResourceManager(Arc<Mutex<HashMap<ResourceAddr, ResourceValue>>>);

pub struct IoManager {
    audio: AudioDevice,
}

impl ResourceManager {
    #[inline(always)]
    pub fn new(_config: &Config) -> Result<Self> {
        Ok(Self(Default::default()))
    }

    pub fn preload_block(
        &mut self,
        resources: Vec<ResourceAddr>,
        tex_manager: Arc<RwLock<epaint::TextureManager>>,
        #[allow(unused)] config: &Config,
        env: &Env,
    ) -> Result<()> {
        // Lock map
        let mut map = self.0.lock().unwrap();

        // Clean up existing resource map
        map.clear();

        // Load default fixation image
        let src = ResourceAddr::Image("fixation.svg".into());
        map.entry(src.clone()).or_insert({
            let tex_manager = tex_manager.clone();
            let (texture, size) = svg_from_bytes(tex_manager, IMAGE_FIXATION, src.path())?;
            ResourceValue::Image(texture, size)
        });
        let mut default_fixation = true;

        // Load default rustacean image
        let src = ResourceAddr::Image("rustacean.svg".into());
        map.entry(src.clone()).or_insert({
            let tex_manager = tex_manager.clone();
            let (texture, size) = svg_from_bytes(tex_manager, IMAGE_RUSTACEAN, src.path())?;
            ResourceValue::Image(texture, size)
        });
        let mut default_rustacean = true;

        // Load resources used in new block
        for src in resources {
            let mut is_new = !map.contains_key(&src);
            match src.path().to_str().unwrap() {
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
                let data = match src.prefix(env.resource()) {
                    ResourceAddr::Ref(path) => ResourceValue::Ref(path),
                    ResourceAddr::Text(path) => {
                        let text = std::fs::read_to_string(&path)
                            .wrap_err_with(|| eyre!("Failed to load text resource ({path:?})"))?;
                        ResourceValue::Text(Arc::new(text))
                    }
                    ResourceAddr::Image(path) => {
                        let tex_manager = tex_manager.clone();
                        let (texture, size) = match src.extension().as_deref() {
                            Some("svg") => {
                                svg_from_file(tex_manager, &path).wrap_err_with(|| {
                                    eyre!("Failed to load SVG resource ({path:?})")
                                })?
                            }
                            _ => image_from_file(tex_manager, &path).wrap_err_with(|| {
                                eyre!("Failed to load image resource ({path:?})")
                            })?,
                        };
                        ResourceValue::Image(texture, size)
                    }
                    ResourceAddr::Audio(path) => ResourceValue::Audio(
                        audio_from_file(&path, config)
                            .wrap_err_with(|| eyre!("Failed to load audio resource ({path:?})"))?,
                    ),
                    ResourceAddr::Video(path) => {
                        let tex_manager = tex_manager.clone();
                        let (frames, framerate) = video_from_file(tex_manager, &path, config)
                            .wrap_err_with(|| eyre!("Failed to load video resource ({path:?})"))?;
                        ResourceValue::Video(frames, framerate)
                    }
                    ResourceAddr::Stream(path) => {
                        let tex_manager = tex_manager.clone();
                        ResourceValue::Stream(
                            stream_from_file(tex_manager, &path, config).wrap_err_with(|| {
                                eyre!("Failed to load stream resource ({path:?})")
                            })?,
                        )
                    }
                };
                println!("+ {src:?} : {data:?}");
                map.insert(src, data);
            }
        }

        Ok(())
    }

    pub fn fetch(&self, src: &ResourceAddr) -> Result<ResourceValue> {
        if let Some(res) = self.0.lock().unwrap().get(src) {
            Ok(res.clone())
        } else {
            Err(eyre!("Tried to fetch unexpected resource: {src:?}"))
        }
    }
}

impl Debug for IoManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "<IO>")
    }
}

impl IoManager {
    pub fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            audio: AudioDevice::new(config)?,
        })
    }

    pub fn try_clone(&self) -> Result<Self> {
        Ok(Self {
            audio: self.audio.try_clone()?,
        })
    }

    pub fn audio(&self) -> Result<AudioSink> {
        self.audio.sink()
    }
}
