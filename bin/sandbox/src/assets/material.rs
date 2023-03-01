use std::ops;
use std::sync::Arc;

use assets_manager::{
    AnyCache,
    Asset,
    BoxedError,
    Compound,
    SharedString,
    loader::{ImageLoader, LoadFrom, TomlLoader}
};
use eyre::Result;
use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Image(Arc<image::DynamicImage>);

impl ops::Deref for Image {
    type Target = image::DynamicImage;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl Asset for Image { type Loader = LoadFrom<image::DynamicImage, ImageLoader>; }

impl From<image::DynamicImage> for Image {
    fn from(value: image::DynamicImage) -> Self {
        Self(Arc::new(value))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TextureSlotDesc {
    Color(Vec3),
    Texture(SharedString),
}

impl Asset for TextureSlotDesc {
    const EXTENSION: &'static str = "toml";
    type Loader = TomlLoader;
}

#[derive(Debug, Clone)]
pub enum TextureSlot {
    Color(Vec3),
    Texture(Image),
}

impl Compound for TextureSlot {
    fn load(cache: AnyCache, id: &SharedString) -> eyre::Result<Self, BoxedError> {
        let desc = cache.load::<TextureSlotDesc>(id)?.cloned();
        Ok(match desc {
            TextureSlotDesc::Color(col) => Self::Color(col),
            TextureSlotDesc::Texture(path) => Self::Texture(
                cache.load(&path)?.cloned(),
            ),
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MaterialDesc {
    pub color: TextureSlotDesc,
    pub normal: Option<SharedString>,
    pub rough_metal: TextureSlotDesc,
}

impl Asset for MaterialDesc {
    const EXTENSION: &'static str = "toml";

    type Loader = TomlLoader;
}

#[derive(Debug, Clone)]
pub struct Material {
    pub color: TextureSlot,
    pub normal: Option<Image>,
    pub rough_metal: TextureSlot,
}

impl Compound for Material {
    fn load(cache: AnyCache, id: &SharedString) -> eyre::Result<Self, BoxedError> {
        fn slot_from_cache(cache: AnyCache, desc: TextureSlotDesc) -> Result<TextureSlot> {
            Ok(match desc {
                TextureSlotDesc::Color(col) => TextureSlot::Color(col),
                TextureSlotDesc::Texture(id) => TextureSlot::Texture(cache.load(&id)?.cloned()),
            })
        }
        let desc = cache.load::<MaterialDesc>(id)?.cloned();
        Ok(Self {
            color: slot_from_cache(cache, desc.color)?,
            normal: if let Some(path) = desc.normal {
                Some(
                    cache.load(&path)?.cloned(),
                )
            } else {
                None
            },
            rough_metal: slot_from_cache(cache, desc.rough_metal)?,
        })
    }
}
