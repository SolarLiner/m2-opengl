use std::ops;
use std::sync::Arc;

use assets_manager::{
    AnyCache,
    Asset, BoxedError, Compound, loader::{ImageLoader, LoadFrom, TomlLoader}, SharedString,
};
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use violette::texture::{SampleMode, Texture, TextureWrap};

#[derive(Debug, Clone)]
pub struct Image {
    pub image: Arc<image::DynamicImage>,
    pub sample_min: (SampleMode, SampleMode),
    pub sample_mag: SampleMode,
    pub wrap_u: TextureWrap,
    pub wrap_v: TextureWrap,
}

impl ops::Deref for Image {
    type Target = image::DynamicImage;

    fn deref(&self) -> &Self::Target {
        self.image.as_ref()
    }
}

impl Asset for Image {
    const EXTENSIONS: &'static [&'static str] = &["jpg", "jpeg", "png"];
    type Loader = LoadFrom<image::DynamicImage, ImageLoader>;
}

impl From<image::DynamicImage> for Image {
    fn from(value: image::DynamicImage) -> Self {
        Self {
            image: Arc::new(value),
            sample_min: (SampleMode::Linear, SampleMode::Linear),
            sample_mag: SampleMode::Linear,
            wrap_u: TextureWrap::ClampEdge,
            wrap_v: TextureWrap::ClampEdge,
        }
    }
}

impl Image {
    pub(crate) fn create_texture_rgb(&self) -> eyre::Result<Texture<[f32; 3]>> {
        let texture = Texture::<[f32; 3]>::from_dynamic_image((*self.image).clone())?;
        texture.generate_mipmaps()?;
        texture.wrap_s(self.wrap_u)?;
        texture.wrap_t(self.wrap_v)?;
        texture.filter_min_mipmap(self.sample_min.0, self.sample_min.1)?;
        texture.filter_mag(self.sample_mag)?;
        Ok(texture)
    }

    pub(crate) fn create_texture_rg(&self) -> eyre::Result<Texture<[f32; 2]>> {
        let texture = Texture::<[f32; 2]>::from_dynamic_image((*self.image).clone())?;
        texture.generate_mipmaps()?;
        texture.wrap_s(self.wrap_u)?;
        texture.wrap_t(self.wrap_v)?;
        texture.filter_min_mipmap(self.sample_min.0, self.sample_min.1)?;
        texture.filter_mag(self.sample_mag)?;
        Ok(texture)
    }
}

const fn default_normal_amount() -> f32 {
    1.
}

const fn default_color_factor() -> Vec3 {
    Vec3::ONE
}

const fn default_rough_metal() -> Vec2 {
    Vec2::ONE
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MaterialDesc {
    #[serde(default)]
    pub transparent: bool,
    pub color: Option<SharedString>,
    #[serde(default = "default_color_factor")]
    pub color_factor: Vec3,
    pub normal: Option<SharedString>,
    #[serde(default = "default_normal_amount")]
    pub normal_amount: f32,
    pub rough_metal: Option<SharedString>,
    #[serde(default = "default_rough_metal")]
    pub rough_metal_factor: Vec2,
}

impl Asset for MaterialDesc {
    const EXTENSION: &'static str = "toml";

    type Loader = TomlLoader;
}

#[derive(Debug, Clone)]
pub struct Material {
    pub transparent: bool,
    pub color: Option<Image>,
    pub color_factor: Vec3,
    pub normal: Option<Image>,
    pub normal_amount: f32,
    pub rough_metal: Option<Image>,
    pub rough_metal_factor: Vec2,
}

impl Compound for Material {
    fn load(cache: AnyCache, id: &SharedString) -> eyre::Result<Self, BoxedError> {
        tracing::debug!(message="Loading material", %id);
        let desc = cache.load::<MaterialDesc>(id)?.cloned();
        Ok(Self {
            transparent: desc.transparent,
            color: desc.color.map(|id| cache.load_owned(id.as_str()).unwrap()),
            color_factor: desc.color_factor,
            normal: if let Some(path) = desc.normal {
                Some(cache.load(&path)?.cloned())
            } else {
                None
            },
            normal_amount: desc.normal_amount,
            rough_metal: desc.rough_metal.map(|id| cache.load_owned(id.as_str()).unwrap()),
            rough_metal_factor: desc.rough_metal_factor,
        })
    }
}
