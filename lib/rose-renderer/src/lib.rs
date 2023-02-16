use std::{collections::HashMap, sync::Weak};

use eyre::Result;
use glam::{UVec2, Vec3};
use rose_core::{
    camera::{Camera, Projection},
    gbuffers::GeometryBuffers,
    light::{GpuLight, Light, LightBuffer},
    material::{Material, Vertex},
    postprocess::Postprocess,
    transform::{Transform, TransformExt, Transformed},
    utils::thread_guard::ThreadGuard,
};
use tracing::span::EnteredSpan;
use violette::{
    buffer::BufferAccess,
    framebuffer::{ClearBuffer, Framebuffer},
};

pub type Mesh = rose_core::mesh::Mesh<rose_core::material::Vertex>;

#[derive(Debug, Clone, Copy)]
pub struct PostprocessInterface {
    pub exposure: f32,
    pub bloom: BloomInterface,
}

#[derive(Debug, Clone, Copy)]
pub struct BloomInterface {
    pub size: f32,
    pub strength: f32,
}

#[derive(Debug)]
pub struct Renderer {
    camera: Camera,
    lights: LightBuffer,
    geom_pass: GeometryBuffers,
    post_process: Postprocess,
    post_process_iface: PostprocessInterface,
    queued_materials: Vec<Weak<Material>>,
    queued_meshes: HashMap<usize, Vec<Transformed<Weak<Mesh>>>>,
    render_span: ThreadGuard<Option<EnteredSpan>>,
}

impl Renderer {
    pub fn new(size: UVec2) -> Result<Self> {
        let mut camera = Camera::default();
        camera.projection.update(size.as_vec2());
        let lights = LightBuffer::new();
        let geom_pass = GeometryBuffers::new(size)?;
        let post_process = Postprocess::new(size)?;
        Ok(Self {
            camera,
            lights,
            geom_pass,
            post_process,
            post_process_iface: PostprocessInterface {
                exposure: 1.,
                bloom: BloomInterface {
                    size: 1e-3,
                    strength: 1e-2,
                },
            },
            queued_materials: vec![],
            queued_meshes: HashMap::default(),
            render_span: ThreadGuard::new(None),
        })
    }

    pub fn post_process_interface(&mut self) -> &mut PostprocessInterface {
        &mut self.post_process_iface
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    #[tracing::instrument]
    pub fn resize(&mut self, size: UVec2) -> Result<()> {
        Framebuffer::backbuffer().viewport(0, 0, size.x as _, size.y as _);
        self.geom_pass.resize(size)?;
        self.post_process.resize(size)?;
        self.camera.projection.update(size.as_vec2());
        Ok(())
    }

    #[tracing::instrument(skip(new_lights))]
    pub fn add_lights(&mut self, new_lights: impl IntoIterator<Item = Light>) -> Result<()> {
        let lights = if !self.lights.is_empty() {
            let existing_lights = GpuLight::download_buffer(&self.lights)?;
            existing_lights.into_iter().map(|gl| gl.into()).collect()
        } else {
            new_lights.into_iter().collect::<Vec<_>>()
        };
        self.lights = GpuLight::create_buffer(lights)?;
        Ok(())
    }

    pub fn begin_render(&mut self) -> Result<()> {
        self.render_span
            .replace(tracing::debug_span!("render").entered());
        let backbuffer = Framebuffer::backbuffer();
        backbuffer.clear_color(Vec3::ZERO.extend(1.).to_array())?;
        backbuffer.clear_depth(1.)?;
        backbuffer.do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH)?;

        self.post_process
            .set_exposure(self.post_process_iface.exposure)?;
        self.post_process
            .set_bloom_size(self.post_process_iface.bloom.size)?;
        self.post_process
            .set_bloom_strength(self.post_process_iface.bloom.strength)?;

        self.geom_pass
            .framebuffer()
            .do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH)?;
        Ok(())
    }

    #[tracing::instrument]
    pub fn submit_mesh(&mut self, material: Weak<Material>, mesh: Transformed<Weak<Mesh>>) {
        let mesh_ptr = Weak::as_ptr(&mesh) as usize;
        let material_ptr = Weak::as_ptr(&material) as usize;
        tracing::debug!(message="Submitting mesh", %mesh_ptr, %material_ptr);
        let mat_ix = if let Some(ix) = self
            .queued_materials
            .iter()
            .position(|mat| mat.ptr_eq(&material))
        {
            ix
        } else {
            let ix = self.queued_materials.len();
            self.queued_materials.push(material);
            ix
        };

        self.queued_meshes
            .entry(mat_ix)
            .and_modify(|v| v.push(mesh.clone()))
            .or_insert_with(|| vec![mesh]);
    }

    #[tracing::instrument]
    pub fn flush(&mut self) -> Result<()> {
        for (mat_ix, meshes) in self.queued_meshes.drain() {
            let Some(material) = self.queued_materials[mat_ix].upgrade() else {
                tracing::warn!("Dropped material value, cannot recover from weakref");
                continue;
            };
            let Some(mut meshes) = meshes.into_iter().map(|w| w.upgrade().map(|v| v.transformed(w.transform))).collect::<Option<Vec<_>>>() else {
                tracing::warn!("Dropped mesh object, cannot recover from weakref");
                continue;
            };

            self.geom_pass
                .draw_meshes(&self.camera, &material, &mut meshes)?;
        }

        self.geom_pass
            .draw_screen(self.post_process.framebuffer(), &self.camera, &self.lights)?;
        self.post_process.draw(&Framebuffer::backbuffer())?;
        self.render_span.take();
        Ok(())
    }
}
