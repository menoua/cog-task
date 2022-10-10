use crate::action::{Action, StatefulAction};
use crate::config::Config;
use crate::error;
use crate::error::Error::{InternalError, InvalidResourceError};
use crate::io::IO;
use crate::resource::{ResourceMap, ResourceValue};
use crate::server::SyncCallback;
use iced::pure::widget::{image, svg, Container};
use iced::pure::Element;
use iced::{ContentFit, Length};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Image {
    src: PathBuf,
    #[serde(default)]
    width: Option<u16>,
    #[serde(default)]
    style: String,
}

impl Action for Image {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![self.src.to_owned()]
    }

    fn stateful(
        &self,
        id: usize,
        res: &ResourceMap,
        _config: &Config,
        _io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        match res.fetch(&self.src)? {
            ResourceValue::Image(image) => Ok(Box::new(StatefulImage {
                id,
                done: false,
                handle: image,
                width: self.width,
            })),
            ResourceValue::Svg(image) => Ok(Box::new(StatefulSvg {
                id,
                done: false,
                handle: image,
                width: self.width,
            })),
            _ => Err(InvalidResourceError(format!(
                "Image action supplied non-image resource: `{:?}`",
                self.src
            ))),
        }
    }
}

#[derive(Debug)]
pub struct StatefulImage {
    id: usize,
    done: bool,
    handle: Arc<image::Handle>,
    width: Option<u16>,
}

impl StatefulAction for StatefulImage {
    #[inline(always)]
    fn id(&self) -> usize {
        self.id
    }

    #[inline(always)]
    fn is_over(&self) -> Result<bool, error::Error> {
        Ok(self.done)
    }

    #[inline(always)]
    fn is_visual(&self) -> bool {
        true
    }

    #[inline(always)]
    fn is_static(&self) -> bool {
        true
    }

    #[inline(always)]
    fn stop(&mut self) -> Result<(), error::Error> {
        self.done = true;
        Ok(())
    }

    fn view(&self, scale_factor: f32) -> Result<Element<'_, SyncCallback>, error::Error> {
        let image = image::Image::new(self.handle.as_ref().clone());
        Ok(if let Some(width) = self.width {
            let width = (scale_factor * width as f32) as u16;
            Container::new(
                Container::new(
                    image
                        .content_fit(ContentFit::Contain)
                        .width(Length::Units(width))
                        .height(Length::Fill),
                )
                .width(Length::Units(width))
                .height(Length::Fill)
                .center_x()
                .center_y(),
            )
        } else {
            Container::new(image.content_fit(ContentFit::ScaleDown))
                .height(Length::Fill)
                .center_x()
                .center_y()
        }
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into())
    }
}

#[derive(Debug)]
pub struct StatefulSvg {
    id: usize,
    done: bool,
    handle: Arc<svg::Handle>,
    width: Option<u16>,
}

impl StatefulAction for StatefulSvg {
    fn id(&self) -> usize {
        self.id
    }

    fn is_over(&self) -> Result<bool, error::Error> {
        Ok(self.done)
    }

    fn is_visual(&self) -> bool {
        true
    }

    fn is_static(&self) -> bool {
        true
    }

    fn stop(&mut self) -> Result<(), error::Error> {
        self.done = true;
        Ok(())
    }

    fn view(&self, scale_factor: f32) -> Result<Element<'_, SyncCallback>, error::Error> {
        let image = svg::Svg::new(self.handle.as_ref().clone());
        Ok(if let Some(width) = self.width {
            let width = (scale_factor * width as f32) as u16;
            Container::new(
                Container::new(
                    image
                        .content_fit(ContentFit::Contain)
                        .width(Length::Units(width))
                        .height(Length::Fill),
                )
                .width(Length::Units(width))
                .height(Length::Fill)
                .center_x()
                .center_y(),
            )
        } else {
            Container::new(image.content_fit(ContentFit::ScaleDown))
                .height(Length::Fill)
                .center_x()
                .center_y()
        }
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fixation {
    #[serde(default)]
    width: Option<u16>,
    #[serde(default)]
    style: String,
}

impl Action for Fixation {
    fn stateful(
        &self,
        _id: usize,
        _res: &ResourceMap,
        _config: &Config,
        _io: &IO,
    ) -> Result<Box<dyn StatefulAction>, error::Error> {
        Err(InternalError(Self::LARVA_PANIC_MSG.to_owned()))
    }

    fn evolve(
        &self,
        _root_dir: &Path,
        _config: &Config,
    ) -> Result<Option<Box<dyn Action>>, error::Error> {
        Ok(Some(Box::new(Image::from(self))))
    }
}

impl Fixation {
    const LARVA_PANIC_MSG: &'static str =
        "Fixation is a larva action, it should be evolved into an \
        Image quickly after initialization";
}

impl From<&Fixation> for Image {
    fn from(f: &Fixation) -> Self {
        Self {
            src: PathBuf::from("fixation.svg"),
            width: f.width,
            style: f.style.clone(),
        }
    }
}

impl From<Fixation> for Image {
    fn from(fixation: Fixation) -> Self {
        Self {
            src: PathBuf::from("fixation.svg"),
            width: fixation.width,
            style: fixation.style,
        }
    }
}
