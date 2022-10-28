pub mod address;
#[cfg(feature = "audio")]
pub mod audio;
pub mod color;
pub mod image;
pub mod io;
pub mod key;
pub mod logger;
pub mod math;
#[cfg(feature = "stream")]
pub mod stream;
pub mod text;
pub mod trigger;
pub mod value;

pub use crate::resource::image::*;
pub use address::*;
#[cfg(feature = "audio")]
pub use audio::*;
pub use color::*;
pub use io::*;
pub use key::*;
pub use logger::*;
pub use math::*;
#[cfg(feature = "stream")]
pub use stream::*;
pub use text::*;
pub use trigger::Trigger;
pub use value::*;

use crate::assets::{IMAGE_FIXATION, IMAGE_RUSTACEAN};
use crate::server::{Config, Env};
use eframe::egui::mutex::RwLock;
use eframe::epaint;
use eyre::{eyre, Result};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

#[derive(Default, Debug, Clone)]
pub struct ResourceMap(Arc<Mutex<HashMap<ResourceAddr, ResourceValue>>>);

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
            let data = ResourceValue::Image(texture, size);
            println!("+ default fixation : {data:?}");
            data
        });
        let mut default_fixation = true;

        // Load default rustacean image
        let src = ResourceAddr::Image("rustacean.svg".into());
        map.entry(src.clone()).or_insert({
            let tex_manager = tex_manager.clone();
            let (texture, size) = svg_from_bytes(tex_manager, IMAGE_RUSTACEAN, src.path())?;
            let data = ResourceValue::Image(texture, size);
            println!("+ default rustacean : {data:?}");
            data
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
                        let text = std::fs::read_to_string(path)?;
                        ResourceValue::Text(Arc::new(text))
                    }
                    ResourceAddr::Image(path) => {
                        let tex_manager = tex_manager.clone();
                        let (texture, size) = match src.extension().as_deref() {
                            Some("svg") => svg_from_file(tex_manager, &path)?,
                            _ => image_from_file(tex_manager, &path)?,
                        };
                        ResourceValue::Image(texture, size)
                    }
                    #[cfg(feature = "audio")]
                    ResourceAddr::Audio(path) => {
                        ResourceValue::Audio(audio_from_file(&path, config)?)
                    }
                    #[cfg(feature = "stream")]
                    ResourceAddr::Video(path) => {
                        let tex_manager = tex_manager.clone();
                        let (frames, framerate) = video_from_file(tex_manager, &path, config)?;
                        ResourceValue::Video(frames, framerate)
                    }
                    #[cfg(feature = "stream")]
                    ResourceAddr::Stream(path) => {
                        let tex_manager = tex_manager.clone();
                        ResourceValue::Stream(stream_from_file(tex_manager, &path, config)?)
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

    pub fn fetch_text(&self, text: &str) -> Result<String> {
        let text: String = match text_or_file(text) {
            Some(src) => {
                let src = ResourceAddr::Text(src);
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
