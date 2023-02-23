use std::{error::Error, num::NonZeroU32};

pub use image;

use crate::{
    math::{Rect, Vec3},
    shader::AsUniform,
    value::AsScalarType,
    base::Resource,
    bind::Bind,
    context::GraphicsContext,
    math::Vec2
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Dimension {
    D1(NonZeroU32),
    D2(Vec2<NonZeroU32>),
    Cube(NonZeroU32),
}

impl Dimension {
    pub fn len(&self) -> u32 {
        let vec = self.to_extents_vector();
        vec.x.get() * vec.y.get() * vec.z.get()
    }

    pub fn to_extents_vector(self) -> Vec3<NonZeroU32> {
        const ONE: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(1) };
        match self {
            Self::D1(w) => Vec3::new(w, ONE, ONE),
            Self::D2(s) => Vec3::new(s.x, s.y, ONE),
            Self::Cube(..) => todo!(),
        }
    }
}

pub trait AsTextureFormat {
    type Subpixel: AsScalarType;
    const NUM_COMPONENTS: u8;
}

impl<T: image::Primitive + AsScalarType> AsTextureFormat for image::Luma<T> {
    type Subpixel = T;
    const NUM_COMPONENTS: u8 = 1;
}

impl<T: image::Primitive + AsScalarType> AsTextureFormat for image::Rgb<T> {
    type Subpixel = T;
    const NUM_COMPONENTS: u8 = 3;
}

impl<T: image::Primitive + AsScalarType> AsTextureFormat for image::Rgba<T> {
    type Subpixel = T;
    const NUM_COMPONENTS: u8 = 4;
}

impl<T: image::Primitive + AsScalarType> AsTextureFormat for T {
    type Subpixel = T;
    const NUM_COMPONENTS: u8 = 1;
}

impl<T: image::Primitive + AsScalarType> AsTextureFormat for [T; 2] {
    type Subpixel = T;
    const NUM_COMPONENTS: u8 = 2;
}

impl<T: image::Primitive + AsScalarType> AsTextureFormat for [T; 3] {
    type Subpixel = T;
    const NUM_COMPONENTS: u8 = 3;
}

impl<T: image::Primitive + AsScalarType> AsTextureFormat for [T; 4] {
    type Subpixel = T;
    const NUM_COMPONENTS: u8 = 4;
}

pub trait Texture<F: AsTextureFormat>: TextureView<F, Texture = Self> + Resource + Bind {
    type Err: Error;
    type Gc: GraphicsContext;
    type View: TextureView<F, Texture = Self>;
    type Uniform: AsUniform<<Self::Gc as GraphicsContext>::ShaderModule>;

    fn resize(&self, extents: Dimension) -> Result<(), Self::Err>;
    fn set_data(&self, data: &[F]) -> Result<(), Self::Err>;
    fn set_data_subpixel(&self, data: &[F::Subpixel]) -> Result<(), Self::Err>;
    fn set_data_rect(&self, rect: Rect<u32>, data: &[F]) -> Result<(), Self::Err>;
    fn set_data_rect_subpixel(&self, rect: Rect<u32>, data: &[F]) -> Result<(), Self::Err>;
    fn read_pixels(
        &self,
        rect: Rect<u32>,
    ) -> Result<image::ImageBuffer<F, Vec<<F as AsTextureFormat>::Subpixel>>, Self::Err>
    where
        F: image::Pixel<Subpixel=<F as AsTextureFormat>::Subpixel>;
    fn upload(
        &self,
        img: &image::ImageBuffer<F, Vec<<F as image::Pixel>::Subpixel>>,
    ) -> Result<(), Self::Err>
    where
        F: image::Pixel<Subpixel=<F as AsTextureFormat>::Subpixel> {
        self.set_data_subpixel(img.as_ref())
    }
    fn get_mipmap(&self, level: usize) -> Result<Self::View, Self::Err>;
}

pub trait TextureView<F: AsTextureFormat>: Send + Sync {
    type Texture: Texture<F, View = Self>;
    fn mipmap(&self) -> usize;
    fn dimensions(&self) -> Dimension;
    fn download(
        &self,
    ) -> Result<
        image::ImageBuffer<F, Vec<<F as AsTextureFormat>::Subpixel>>,
        <Self::Texture as Texture<F>>::Err,
    >
    where
        F: image::Pixel;
}
