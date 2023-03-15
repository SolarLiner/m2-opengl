use std::sync::RwLock;

use crevice::std140::AsStd140;
use eyre::{Context, Result};
use glam::{IVec4, UVec4, Vec2, Vec3, Vec4};

use rose_core::{
    camera::ViewUniformBuffer,
    transform::Transformed,
    utils::reload_watcher::{ReloadFileProxy, ReloadWatcher},
};
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
use crate::{bones::Std140GpuBone, DrawMaterial};

#[derive(Debug, Default, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, VertexAttributes)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub bones_ix: IVec4,
    pub bones_weights: Vec4,
}

impl Vertex {
    pub const fn new(position: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Self {
            position,
            normal,
            uv,
            bones_ix: IVec4::splat(-1),
            bones_weights: Vec4::ZERO,
        }
    }

    pub fn attach_bones(mut self, bones_ix: UVec4, weights: Vec4) -> Self {
        self.bones_ix = bones_ix.as_ivec4();
        self.bones_weights = weights;
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
    pub has_emission: bool,
    pub emission_factor: Vec3,
}

#[derive(Debug)]
pub struct Material {
    program: RwLock<Program>,
    u_color: UniformLocation,
    u_normal: UniformLocation,
    u_rough_metal: UniformLocation,
    u_model: UniformLocation,
    u_uniforms: UniformBlockIndex,
    u_view: UniformBlockIndex,
    u_bones: UniformBlockIndex,
    bones_uniform: UniformBuffer<Std140GpuBone>,
    reload_watcher: ReloadFileProxy,
    u_emission: UniformLocation,
}

impl Material {
    pub fn create(
        camera_uniform: Option<&ViewUniformBuffer>,
        reload_watcher: &ReloadWatcher,
    ) -> Result<Self> {
        let vert_path = reload_watcher.base_path().join("mesh/mesh.vert.glsl");
        let frag_path = reload_watcher.base_path().join("mesh/mesh.frag.glsl");
        let vert_files = glsl_preprocessor::load_and_parse(vert_path)
            .with_context(|| "Parsing mesh vertex shader")?;
        let frag_files = glsl_preprocessor::load_and_parse(frag_path)
            .with_context(|| "Parsing mesh fragment shader")?;
        let vert_shader = VertexShader::new_multiple(vert_files.iter().map(|(_, s)| s.as_str()))
            .with_context(|| {
                format!(
                    "File map:\n{}",
                    vert_files
                        .iter()
                        .map(|(p, _)| p.as_path())
                        .enumerate()
                        .map(|(ix, p)| format!("\t{} => {}", ix, p.display()))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })?;
        let frag_shader = FragmentShader::new_multiple(frag_files.iter().map(|(_, s)| s.as_str()))
            .with_context(|| {
                format!(
                    "File map:\n{}",
                    frag_files
                        .iter()
                        .map(|(p, _)| p.as_path())
                        .enumerate()
                        .map(|(ix, p)| format!("\t{} => {}", ix, p.display()))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })?;
        let program = Program::new()
            .with_shader(vert_shader.id)
            .with_shader(frag_shader.id)
            .link()?;
        let u_color = program.uniform("map_color");
        let u_normal = program.uniform("map_normal");
        let u_rough_metal = program.uniform("map_rough_metal");
        let u_emission = program.uniform("map_emission");
        let u_uniforms = program.uniform_block("Uniforms");
        let u_model = program.uniform("model");
        let u_view = program.uniform_block("View");
        let u_bones = program.uniform_block("Bones");

        if let Some(buf) = camera_uniform {
            program.bind_block(&buf.slice(0..=0), u_view, 0)?;
        }
        Ok(Self {
            program: RwLock::new(program),
            u_color,
            u_normal,
            u_rough_metal,
            u_emission,
            u_model,
            u_uniforms,
            u_view,
            u_bones,
            bones_uniform: UniformBuffer::new(),
            reload_watcher: reload_watcher.proxy(
                vert_files
                    .iter()
                    .map(|(p, _)| p.as_path())
                    .chain(frag_files.iter().map(|(p, _)| p.as_path())),
            ),
        })
    }

    pub fn draw_meshes<'a>(
        &mut self,
        frame: &Framebuffer,
        _view: &ViewUniformBuffer,
        instance: &MaterialInstance,
        meshes: impl IntoIterator<Item = Transformed<&'a Mesh>>,
    ) -> Result<()> {
        {
            if self.reload_watcher.should_reload() {
                let mut paths = self.reload_watcher.paths();
                let vert_path = paths.next().unwrap();
                let frag_path = paths.next().unwrap();
                tracing::debug!(message="Reloading material shader", vert=%vert_path.display(), frag=%frag_path.display());
                let vert_shader = VertexShader::load(vert_path)?;
                let frag_shader = FragmentShader::load(frag_path)?;
                *self.program.write().unwrap() = Program::new()
                    .with_shader(vert_shader.id)
                    .with_shader(frag_shader.id)
                    .link()?;
            }
        }
        let program = self.program();
        program.bind_block(&instance.buffer.slice(0..=0), self.u_uniforms, 1)?;
        program.bind_block(&self.bones_uniform.slice(..), self.u_bones, 2)?;
        if let Some(color) = instance.color.as_ref() {
            program.set_uniform(self.u_color, color.as_uniform(0)?)?;
        }
        if let Some(normal) = &instance.normal_map {
            program.set_uniform(self.u_normal, normal.as_uniform(1)?)?;
        }
        if let Some(rough_metal) = &instance.roughness_metal {
            program.set_uniform(self.u_rough_metal, rough_metal.as_uniform(2)?)?;
        }
        if let Some(emission) = &instance.emission {
            program.set_uniform(self.u_emission, emission.as_uniform(3)?)?;
        }
        drop(program);

        for mesh in meshes {
            if let Some(root_bone) = &mesh.root_bone {
                root_bone.update_buffer(&mut self.bones_uniform)?;
            }
            let program = self.program();
            program.set_uniform(self.u_model, mesh.transform.matrix())?;
            mesh.draw(&program, frame, false)?;
        }
        unsafe { gl::BindTexture(gl::TEXTURE_2D, 0) }
        Ok(())
    }

    pub fn set_camera_uniform(&self, buffer: &ViewUniformBuffer) -> Result<()> {
        self.program()
            .bind_block(&buffer.slice(0..=0), self.u_view, 0)?;
        Ok(())
    }

    fn program(&self) -> impl '_ + Drop + std::ops::Deref<Target = Program> {
        self.program.read().unwrap()
    }
}

#[derive(Debug)]
pub struct MaterialInstance {
    pub color: Option<Texture<[f32; 3]>>,
    pub normal_map: Option<Texture<[f32; 3]>>,
    pub roughness_metal: Option<Texture<[f32; 2]>>,
    pub emission: Option<Texture<[f32; 3]>>,
    uniforms: MaterialUniforms,
    buffer: UniformBuffer<Std140MaterialUniforms>,
}

impl MaterialInstance {
    pub fn create(
        color_slot: impl Into<Option<Texture<[f32; 3]>>>,
        normal_map: impl Into<Option<Texture<[f32; 3]>>>,
        rough_metal: impl Into<Option<Texture<[f32; 2]>>>,
        emission: impl Into<Option<Texture<[f32; 3]>>>,
    ) -> Result<Self> {
        let color = color_slot.into();
        let normal_map = normal_map.into();
        let roughness_metal = rough_metal.into();
        let emission = emission.into();
        let uniforms = MaterialUniforms {
            has_color: color.is_some(),
            color_factor: Vec3::ONE,
            has_normal: normal_map.is_some(),
            normal_amount: 1.,
            has_rough_metal: roughness_metal.is_some(),
            rough_metal_factor: Vec2::ONE,
            has_emission: emission.is_some(),
            emission_factor: Vec3::ZERO,
        };
        let buffer = UniformBuffer::with_data(&[uniforms.as_std140()])?;
        Ok(Self {
            color,
            normal_map,
            roughness_metal,
            emission,
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
