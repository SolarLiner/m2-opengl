use std::fmt::Formatter;
use std::{fmt, marker::PhantomData, num::NonZeroU32};

use gl::types::GLenum;
use violette_api::base::Resource;
use violette_api::{
    bind::Bind,
    math::Rect,
    texture::image::{ImageBuffer, Pixel},
    texture::{AsTextureFormat, Dimension, Texture as ApiTexture, TextureView as ApiTextureView},
    value::{AsScalarType, ScalarType},
};

use crate::{api::OpenGLError, context::OpenGLContext, get_ext_label, set_ext_label, Gl, GlObject};

fn gl_dimension_target(dim: Dimension) -> u32 {
    match dim {
        Dimension::D1(_) => gl::TEXTURE_1D,
        Dimension::D2(_) => gl::TEXTURE_2D,
        Dimension::Cube(_) => gl::TEXTURE_CUBE_MAP,
    }
}

fn scalar_type_int(typ: ScalarType) -> bool {
    use ScalarType::*;
    match typ {
        I8 | I16 | I32 => true,
        _ => false,
    }
}

fn gl_format<F: AsTextureFormat>() -> u32 {
    if scalar_type_int(F::Subpixel::scalar_type()) {
        match F::NUM_COMPONENTS {
            1 => gl::RED_INTEGER,
            2 => gl::RG_INTEGER,
            3 => gl::RGB_INTEGER,
            4 => gl::RGBA_INTEGER,
            _ => unreachable!(),
        }
    } else {
        match F::NUM_COMPONENTS {
            1 => gl::RED,
            2 => gl::RG,
            3 => gl::RGB,
            4 => gl::RGBA,
            _ => unreachable!(),
        }
    }
}

fn gl_internal_format<F: AsTextureFormat>() -> GLenum {
    match F::NUM_COMPONENTS {
        1 => match F::Subpixel::scalar_type() {
            ScalarType::Bool => gl::R8UI,
            ScalarType::I8 => gl::R8I,
            ScalarType::I16 => gl::R16I,
            ScalarType::I32 => gl::R32I,
            ScalarType::U8 => gl::R8UI,
            ScalarType::U16 => gl::R16UI,
            ScalarType::U32 => gl::R32UI,
            ScalarType::F32 => gl::R32F,
            _ => unreachable!(),
        },
        2 => match F::Subpixel::scalar_type() {
            ScalarType::Bool => gl::RG8UI,
            ScalarType::I8 => gl::RG8I,
            ScalarType::I16 => gl::RG16I,
            ScalarType::I32 => gl::RG32I,
            ScalarType::U8 => gl::RG8UI,
            ScalarType::U16 => gl::RG16UI,
            ScalarType::U32 => gl::RG32UI,
            ScalarType::F32 => gl::RG32F,
            _ => unreachable!(),
        },
        3 => match F::Subpixel::scalar_type() {
            ScalarType::Bool => gl::RGB8UI,
            ScalarType::I8 => gl::RGB8I,
            ScalarType::I16 => gl::RGB16I,
            ScalarType::I32 => gl::RGB32I,
            ScalarType::U8 => gl::RGB8UI,
            ScalarType::U16 => gl::RGB16UI,
            ScalarType::U32 => gl::RGB32UI,
            ScalarType::F32 => gl::RGB32F,
            _ => unreachable!(),
        },
        4 => match F::Subpixel::scalar_type() {
            ScalarType::Bool => gl::RGBA8UI,
            ScalarType::I8 => gl::RGBA8I,
            ScalarType::I16 => gl::RGBA16I,
            ScalarType::I32 => gl::RGBA32I,
            ScalarType::U8 => gl::RGBA8UI,
            ScalarType::U16 => gl::RGBA16UI,
            ScalarType::U32 => gl::RGBA32UI,
            ScalarType::F32 => gl::RGBA32F,
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

fn gl_type(typ: ScalarType) -> u32 {
    match typ {
        ScalarType::Bool => gl::BOOL,
        ScalarType::I8 => gl::BYTE,
        ScalarType::I16 => gl::SHORT,
        ScalarType::I32 => gl::INT,
        ScalarType::U8 => gl::UNSIGNED_BYTE,
        ScalarType::U16 => gl::UNSIGNED_SHORT,
        ScalarType::U32 => gl::UNSIGNED_INT,
        ScalarType::F32 => gl::FLOAT,
        ScalarType::F64 => gl::DOUBLE,
        _ => unreachable!("Cannot use {:?} in OpenGL for texture types", typ),
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TextureId {
    pub id: NonZeroU32,
    pub target: u32,
}

pub struct Texture<F> {
    __phantom: PhantomData<F>,
    gl: Gl,
    dimension: Option<Dimension>,
    id: NonZeroU32,
}

impl<F> fmt::Debug for Texture<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple(&format!("Texture<{}>", std::any::type_name::<F>()))
            .field(&self.id.get())
            .finish()
    }
}

impl<F> Texture<F> {
    pub(crate) fn new(gl: &Gl) -> Self {
        let mut id = 0;
        unsafe {
            gl.GenTextures(1, &mut id);
        }
        let id = NonZeroU32::new(id as _).unwrap();
        Self {
            __phantom: PhantomData,
            gl: gl.clone(),
            dimension: None,
            id,
        }
    }

    pub(crate) fn reserve_memory(&self, dimensions: Dimension) -> Result<(), OpenGLError>
    where
        F: AsTextureFormat,
    {
        match dimensions {
            Dimension::D1(w) => unsafe {
                self.gl.TexImage1D(
                    gl::TEXTURE_1D,
                    0,
                    gl_internal_format::<F>() as _,
                    w.get() as _,
                    0,
                    gl_format::<F>(),
                    gl_type(F::Subpixel::scalar_type()),
                    std::ptr::null(),
                );
            },
            Dimension::D2(s) => unsafe {
                self.gl.TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl_internal_format::<F>() as _,
                    s.x.get() as _,
                    s.y.get() as _,
                    0,
                    gl_format::<F>(),
                    gl_type(F::Subpixel::scalar_type()),
                    std::ptr::null(),
                );
            },
            Dimension::Cube(_) => todo!("Not implemented: cube map textures"),
        }
        OpenGLError::guard(&self.gl)
    }

    pub(crate) fn generate_mipmaps(&self) -> Result<(), OpenGLError> {
        if let Some(dim) = self.dimension {
            unsafe {
                self.gl.GenerateMipmap(gl_dimension_target(dim));
            }
            OpenGLError::guard(&self.gl)?;
        }
        Ok(())
    }
}

impl<F> Bind for Texture<F> {
    type Id = TextureId;

    fn id(&self) -> Self::Id {
        TextureId {
            id: self.id,
            target: self
                .dimension
                .map(gl_dimension_target)
                .unwrap_or(gl::TEXTURE_2D),
        }
    }

    fn bind(&self) {
        let target = self
            .dimension
            .map(gl_dimension_target)
            .unwrap_or(gl::TEXTURE_2D);
        unsafe {
            self.gl.BindTexture(target, self.id.get());
        }
    }

    fn unbind(&self) {
        let target = self
            .dimension
            .map(gl_dimension_target)
            .unwrap_or(gl::TEXTURE_2D);
        unsafe {
            self.gl.BindTexture(target, 0);
        }
    }
}

impl<F> GlObject for Texture<F> {
    const GL_NAME: GLenum = gl::TEXTURE;

    fn gl(&self) -> &Gl {
        &self.gl
    }

    fn id(&self) -> u32 {
        self.id.get()
    }
}

impl<F: Send + Sync> Resource for Texture<F> {
    fn set_name(&self, name: impl ToString) {
        set_ext_label(self, name)
    }

    fn get_name(&self) -> Option<String> {
        get_ext_label(self)
    }
}

impl<F: AsTextureFormat> ApiTexture<F> for Texture<F> {
    type Err = OpenGLError;
    type Gc = OpenGLContext;
    type View = ();
    type Uniform = TextureUnit;

    fn resize(&self, extents: Dimension) -> Result<(), Self::Err> {
        self.reserve_memory(extents)
    }

    fn set_data(&self, data: &[F]) -> Result<(), Self::Err> {
        let dimension = self
            .dimension
            .expect("No dimension set for this texture; first resize it");
        let extents = dimension.len();
        let given_len = data.len() * F::NUM_COMPONENTS as _;
        debug_assert_eq!(
            extents, given_len,
            "Texture data size must match the texture size"
        );
        unsafe {
            match dimension {
                Dimension::D1(w) => self.gl.TexImage1D(
                    gl::TEXTURE_1D,
                    0,
                    gl_internal_format::<F>() as _,
                    w.get() as _,
                    0,
                    gl_format::<F>(),
                    gl_type(F::Subpixel::scalar_type()),
                    data.as_ptr(),
                ),
                Dimension::D2(s) => self.gl.TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl_internal_format::<F>() as _,
                    s.x.get() as _,
                    s.y.get() as _,
                    0,
                    gl_format::<F>(),
                    gl_type(F::Subpixel::scalar_type()),
                    data.as_ptr(),
                ),
                Dimension::Cube(_) => {}
            }
        }
        OpenGLError::guard(&self.gl)
    }

    fn set_data_subpixel(&self, data: &[F::Subpixel]) -> Result<(), Self::Err> {
        todo!()
    }

    fn set_data_rect(&self, rect: Rect<u32>, data: &[F]) -> Result<(), Self::Err> {
        todo!()
    }

    fn set_data_rect_subpixel(&self, rect: Rect<u32>, data: &[F]) -> Result<(), Self::Err> {
        todo!()
    }

    fn read_pixels(
        &self,
        rect: Rect<u32>,
    ) -> Result<ImageBuffer<F, Vec<<F as AsTextureFormat>::Subpixel>>, Self::Err>
    where
        F: Pixel,
    {
        todo!()
    }

    fn get_mipmap(&self, level: usize) -> Result<Self::View, Self::Err> {
        todo!()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TextureUnit(u32);
