use std::path::Path;

use crevice::std140::AsStd140;
use eyre::Result;
use glam::{IVec4, UVec4, Vec2, Vec3};

use rose_core::camera::ViewUniformBuffer;
use rose_core::transform::Transformed;
use violette::{
    buffer::UniformBuffer,
    framebuffer::Framebuffer,
    gl,
    program::{Program, UniformBlockIndex, UniformLocation},
    shader::{FragmentShader, VertexShader},
    texture::Texture,
};
use violette_derive::VertexAttributes;

use crate::Mesh;

#[derive(Debug, Default, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, VertexAttributes)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub bones_ix: IVec4,
}

impl Vertex {
    pub fn new(position: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Self {
            position,
            normal,
            uv,
            bones_ix: IVec4::splat(-1),
        }
    }

    pub fn attach_bones(mut self, bones_ix: UVec4) -> Self {
        self.bones_ix = bones_ix.as_ivec4();
        self
    }
}

#[derive(Debug, Copy, Clone, AsStd140)]
pub struct MaterialUniforms {
    pub has_color: bool,
    pub color_factor: Vec3,
    pub has_normal: bool,
    pub normal_amount: f32,
    pub has_rough_metal: bool,
    pub rough_metal_factor: Vec2,
}

#[derive(Debug)]
pub struct Material {
    program: Program,
    u_color: UniformLocation,
    u_normal: UniformLocation,
    u_rough_metal: UniformLocation,
    u_model: UniformLocation,
    u_uniforms: UniformBlockIndex,
    u_view: UniformBlockIndex,
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
        let u_color = program.uniform("map_color");
        let u_normal = program.uniform("map_normal");
        let u_rough_metal = program.uniform("map_rough_metal");
        let u_uniforms = program.uniform_block("Uniforms");
        let u_model = program.uniform("model");
        let u_view = program.uniform_block("View");

        if let Some(buf) = camera_uniform {
            program.bind_block(&buf.slice(0..=0), u_view, 0)?;
        }
        Ok(Self {
            program,
            u_color,
            u_normal,
            u_rough_metal,
            u_model,
            u_uniforms,
            u_view,
        })
    }

    pub fn set_camera_uniform(&self, buffer: &ViewUniformBuffer) -> Result<()> {
        self.program
            .bind_block(&buffer.slice(0..=0), self.u_view, 0)?;
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
            .bind_block(&instance.buffer.slice(0..=0), self.u_uniforms, 1)?;
        if let Some(color) = instance.color.as_ref() {
            self.program
                .set_uniform(self.u_color, color.as_uniform(0)?)?;
        }
        if let Some(normal) = &instance.normal_map {
            self.program
                .set_uniform(self.u_normal, normal.as_uniform(1)?)?;
        }
        if let Some(rough_metal) = &instance.roughness_metal {
            self.program
                .set_uniform(self.u_rough_metal, rough_metal.as_uniform(2)?)?;
        }

        for mesh in meshes {
            self.program
                .set_uniform(self.u_model, mesh.transform.matrix())?;
            mesh.draw(&self.program, framebuffer, false)?;
        }
        unsafe { gl::BindTexture(gl::TEXTURE_2D, 0) }
        Ok(())
    }
}

#[derive(Debug)]
pub struct MaterialInstance {
    pub color: Option<Texture<[f32; 3]>>,
    pub normal_map: Option<Texture<[f32; 3]>>,
    pub roughness_metal: Option<Texture<[f32; 2]>>,
    uniforms: MaterialUniforms,
    buffer: UniformBuffer<Std140MaterialUniforms>,
}

impl MaterialInstance {
    pub fn create(
        color_slot: impl Into<Option<Texture<[f32; 3]>>>,
        normal_map: impl Into<Option<Texture<[f32; 3]>>>,
        rough_metal: impl Into<Option<Texture<[f32; 2]>>>,
    ) -> Result<Self> {
        let color = color_slot.into();
        let normal_map = normal_map.into();
        let roughness_metal = rough_metal.into();
        let uniforms = MaterialUniforms {
            has_color: color.is_some(),
            color_factor: Vec3::ONE,
            has_normal: normal_map.is_some(),
            normal_amount: 1.,
            has_rough_metal: roughness_metal.is_some(),
            rough_metal_factor: Vec2::ONE,
        };
        let buffer = UniformBuffer::with_data(&[uniforms.as_std140()])?;
        Ok(Self {
            color,
            normal_map,
            roughness_metal,
            uniforms,
            buffer,
        })
    }

    pub fn uniforms(&self) -> MaterialUniforms {
        self.uniforms
    }

    pub fn update_uniforms(&mut self, func: impl FnOnce(&mut MaterialUniforms)) -> Result<()> {
        func(&mut self.uniforms);
        let mut slice = self.buffer.slice(0..=0);
        slice.set(0, &self.uniforms.as_std140())?;
        Ok(())
    }
}
