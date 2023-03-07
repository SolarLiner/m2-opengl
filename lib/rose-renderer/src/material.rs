use std::num::NonZeroU32;
use std::path::Path;

use eyre::Result;
use glam::{Vec2, Vec3};

use rose_core::camera::ViewUniformBuffer;
use rose_core::transform::Transformed;
use violette::{
    framebuffer::Framebuffer,
    gl,
    program::{Program, UniformLocation},
    shader::VertexShader,
    texture::Texture,
};
use violette::program::UniformBlockIndex;
use violette::shader::FragmentShader;
use violette::texture::{Dimension, TextureFormat};
use violette_derive::VertexAttributes;

use crate::Mesh;

#[derive(Debug, Default, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, VertexAttributes)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

// impl VertexAttributes for Vertex {
//     fn attributes() -> &'static [VertexDesc] {
//         vec![
//             VertexDesc::from_gl_type::<Vec3>(offset_of!(Self, position)),
//             VertexDesc::from_gl_type::<Vec3>(offset_of!(Self, normal)),
//             VertexDesc::from_gl_type::<Vec2>(offset_of!(Self, uv)),
//         ]
//         .leak()
//     }
// }

impl Vertex {
    pub fn new(position: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Self {
            position,
            normal,
            uv,
        }
    }
}

#[derive(Debug)]
pub enum TextureSlot<const N: usize> {
    Texture(Texture<[f32; N]>),
    Color([f32; N]),
}

impl<const N: usize> From<Texture<[f32; N]>> for TextureSlot<N> {
    fn from(v: Texture<[f32; N]>) -> Self {
        Self::Texture(v)
    }
}

impl<const N: usize> From<[f32; N]> for TextureSlot<N> {
    fn from(v: [f32; N]) -> Self {
        Self::Color(v)
    }
}

impl<const N: usize> TryInto<Texture<[f32; N]>> for TextureSlot<N>
where
    [f32; N]: TextureFormat<Subpixel = f32>,
{
    type Error = eyre::Report;

    fn try_into(self) -> Result<Texture<[f32; N]>> {
        Ok(match self {
            Self::Texture(tex) => tex,
            Self::Color(col) => {
                const ONE: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(1) };
                let tex = Texture::new(ONE, ONE, ONE, Dimension::D2);
                tex.set_data(&col)?;
                tex
            }
        })
    }
}

#[derive(Debug)]
pub struct Material {
    program: Program,
    uniform_color: UniformLocation,
    uniform_normal: UniformLocation,
    uniform_normal_enabled: UniformLocation,
    uniform_normal_amt: UniformLocation,
    uniform_rough_metal: UniformLocation,
    uniform_view: UniformBlockIndex,
    uniform_model: UniformLocation,
}

impl Material {
    pub fn create(camera_uniform: Option<&ViewUniformBuffer>) -> Result<Self> {
        let shaders_dir = Path::new("assets").join("shaders");
        let vert_shader = VertexShader::load(shaders_dir.join("mesh.vert.glsl"))?;
        let frag_shader = FragmentShader::load(shaders_dir.join("mesh.frag.glsl"))?;
        let program = Program::new()
            .with_shader(vert_shader.id)
            .with_shader(frag_shader.id)
            .link()?;
        let uniform_color = program.uniform("color").unwrap();
        let uniform_normal = program.uniform("normal_map").unwrap();
        let uniform_normal_amt = program.uniform("normal_amount").unwrap();
        let uniform_normal_enabled = program.uniform("normal_enabled").unwrap();
        let uniform_rough_metal = program.uniform("rough_metal").unwrap();
        let uniform_model = program.uniform("model").unwrap();
        let uniform_view = program.uniform_block("View", 0).unwrap();
        if let Some(buf) = camera_uniform {
            program.bind_block(uniform_view, &buf.slice(0..=0))?;
        }
        Ok(Self {
            program,
            uniform_color,
            uniform_normal,
            uniform_normal_amt,
            uniform_normal_enabled,
            uniform_rough_metal,
            uniform_model,
            uniform_view,
        })
    }

    pub fn set_camera_uniform(&self, buffer: &ViewUniformBuffer) -> Result<()> {
        self.program.bind_block(self.uniform_view, &buffer.slice(0..=0))?;
        Ok(())
    }

    #[tracing::instrument(skip(self, meshes), fields(meshes = meshes.len()))]
    pub fn draw_meshes<MC: std::ops::Deref<Target=Mesh>>(
        &self,
        framebuffer: &Framebuffer,
        instance: &MaterialInstance,
        meshes: &[Transformed<MC>],
    ) -> Result<()> {
        self.program
            .set_uniform(self.uniform_color, instance.color.as_uniform(0)?)?;
        if let Some(normal) = &instance.normal_map {
            self.program
                .set_uniform(self.uniform_normal, normal.as_uniform(1)?)?;
            self.program.set_uniform(self.uniform_normal_amt, instance.normal_map_amount)?;
            self.program.set_uniform(self.uniform_normal_enabled, 1)?;
        } else {
            self.program.set_uniform(self.uniform_normal_enabled, 0)?;
        }
        self.program
            .set_uniform(self.uniform_rough_metal, instance.roughness_metal.as_uniform(2)?)?;

        for mesh in meshes {
            self.program
                .set_uniform(self.uniform_model, mesh.transform.matrix())?;
            mesh.draw(&self.program, framebuffer, false)?;
        }
        unsafe { gl::BindTexture(gl::TEXTURE_2D, 0) }
        Ok(())
    }
}

#[derive(Debug)]
pub struct MaterialInstance {
    pub color: Texture<[f32; 3]>,
    pub normal_map: Option<Texture<[f32; 3]>>,
    pub normal_map_amount: f32,
    pub roughness_metal: Texture<[f32; 2]>,
}

impl MaterialInstance {
    pub fn create(
        color_slot: impl Into<TextureSlot<3>>,
        normal_map: impl Into<Option<Texture<[f32; 3]>>>,
        rough_metal: impl Into<TextureSlot<2>>,
    ) -> Result<Self> {
        Ok(Self {
            color: color_slot.into().try_into()?,
            normal_map: normal_map.into(),
            roughness_metal: rough_metal.into().try_into()?,
            normal_map_amount: 1.,
        })
    }

    pub fn with_normal_amount(mut self, normal: f32) -> Self {
        self.normal_map_amount = normal;
        self
    }
}