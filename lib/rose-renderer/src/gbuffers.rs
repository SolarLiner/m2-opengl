use std::num::NonZeroU32;
use glam::UVec2;

use eyre::{Context, Result};

use violette::{
    base::resource::Resource,
    framebuffer::{
        Blend,
        ClearBuffer,
        Framebuffer,
        DepthTestFunction,
    },
    program::{UniformBlockIndex, UniformLocation},
    texture::{DepthStencil, Dimension, SampleMode, Texture},
};

use rose_core::{
    light::LightBuffer,
    screen_draw::ScreenDraw
};
use rose_core::camera::Camera;
use rose_core::material::{Material, Vertex};
use rose_core::mesh::Mesh;
use rose_core::transform::Transformed;

#[derive(Debug)]
pub struct GeometryBuffers {
    screen_pass: ScreenDraw,
    debug_texture: ScreenDraw,
    deferred_fbo: Framebuffer,
    output_fbo: Framebuffer,
    pos: Texture<[f32; 3]>,
    albedo: Texture<[f32; 3]>,
    normal: Texture<[f32; 3]>,
    rough_metal: Texture<[f32; 2]>,
    out_color: Texture<[f32; 3]>,
    out_depth: Texture<DepthStencil<f32, ()>>,
    uniform_camera_pos: UniformLocation,
    uniform_frame_pos: UniformLocation,
    uniform_frame_albedo: UniformLocation,
    uniform_frame_normal: UniformLocation,
    uniform_frame_rough_metal: UniformLocation,
    uniform_block_light: UniformBlockIndex,
    debug_uniform_in_texture: UniformLocation,
}

impl GeometryBuffers {
    pub fn new(size: UVec2) -> Result<Self> {
        let Some(width) = NonZeroU32::new(size.x) else { eyre::bail!("Zero width resize"); };
        let Some(height) = NonZeroU32::new(size.y) else { eyre::bail!("Zero height resize"); };
        let nonzero_one = NonZeroU32::new(1).unwrap();
        let pos = Texture::new(width, height, nonzero_one, Dimension::D2);
            pos.filter_min(SampleMode::Linear)?;
            pos.filter_mag(SampleMode::Linear)?;
            pos.reserve_memory()?;

        let albedo = Texture::new(width, height, nonzero_one, Dimension::D2);
        albedo.filter_min(SampleMode::Linear)?;
        albedo.filter_mag(SampleMode::Linear)?;
        albedo.reserve_memory()?;

        let normal = Texture::new(width, height, nonzero_one, Dimension::D2);
        normal.filter_min(SampleMode::Linear)?;
        normal.filter_mag(SampleMode::Linear)?;
        normal.reserve_memory()?;

        let rough_metal = Texture::new(width, height, nonzero_one, Dimension::D2);
        rough_metal.filter_min(SampleMode::Linear)?;
        rough_metal.filter_mag(SampleMode::Linear)?;
        rough_metal.reserve_memory()?;

        let out_color = Texture::new(width, height, nonzero_one, Dimension::D2);
        out_color.filter_min(SampleMode::Linear)?;
        out_color.filter_mag(SampleMode::Linear)?;
        out_color.reserve_memory()?;

        let out_depth = Texture::new(width, height, nonzero_one, Dimension::D2);
        out_depth.filter_min(SampleMode::Linear)?;
        out_depth.filter_mag(SampleMode::Linear)?;
        out_depth.reserve_memory()?;

        let deferred_fbo = Framebuffer::new();
        deferred_fbo.attach_color(0, &pos)?;
        deferred_fbo.attach_color(1, &albedo)?;
        deferred_fbo.attach_color(2, &normal)?;
        deferred_fbo.attach_color(3, &rough_metal)?;
        deferred_fbo.attach_depth(&out_depth)?;
        deferred_fbo.assert_complete()?;
        deferred_fbo.clear_color([0., 0., 0., 1.])?;
        deferred_fbo.clear_depth(1.)?;
        deferred_fbo.viewport(0, 0, size.x as _, size.y as _);

        let output_fbo = Framebuffer::new();
        output_fbo.attach_color(0, &out_color)?;
        output_fbo.assert_complete()?;
        output_fbo.disable_depth_test()?;

        let screen_pass = ScreenDraw::load("assets/shaders/defferred.frag.glsl")
            .context("Cannot load screen shader pass")?;
        let debug_texture = ScreenDraw::load("assets/shaders/blit.frag.glsl")
            .context("Cannot load blit program")?;
        let debug_uniform_in_texture = debug_texture.uniform("in_texture").unwrap();

        let uniform_camera_pos = screen_pass.uniform("camera_pos").unwrap();
        let uniform_frame_pos = screen_pass.uniform("frame_position").unwrap();
        let uniform_frame_albedo = screen_pass.uniform("frame_albedo").unwrap();
        let uniform_frame_normal = screen_pass.uniform("frame_normal").unwrap();
        let uniform_frame_rough_metal = screen_pass.uniform("frame_rough_metal").unwrap();
        let uniform_block_light = screen_pass.uniform_block("Light", 0).unwrap();

        Ok(Self {
            deferred_fbo,
            output_fbo,
            pos,
            albedo,
            normal,
            rough_metal,
            out_color,
            out_depth,
            uniform_camera_pos,
            debug_uniform_in_texture,
            uniform_frame_pos,
            uniform_frame_albedo,
            uniform_frame_normal,
            uniform_frame_rough_metal,
            uniform_block_light,
            screen_pass,
            debug_texture,
        })
    }

    pub fn framebuffer(&self) -> &Framebuffer {
        &self.deferred_fbo
    }

    #[tracing::instrument(skip_all)]
    pub fn draw_meshes<MC: std::ops::Deref<Target=Mesh<Vertex>>>(
        &self,
        camera: &Camera,
        material: &Material,
        meshes: &mut [Transformed<MC>],
    ) -> Result<()> {
        self.deferred_fbo.disable_blending()?;
        self.deferred_fbo.disable_scissor()?;
        self.deferred_fbo.enable_depth_test(DepthTestFunction::Less)?;
        self.deferred_fbo.enable_buffers([0, 1, 2, 3])?;
        material.draw_meshes(&self.deferred_fbo, camera, meshes)?;

        Ok(())
    }

    pub fn debug_position(&self, frame: &Framebuffer) -> Result<()> {
        let unit = self.pos.as_uniform(0)?;
        self.debug_texture.set_uniform(self.debug_uniform_in_texture, unit)?;
        self.debug_texture.draw(frame)?;
        Ok(())
    }

    pub fn debug_albedo(&self, frame: &Framebuffer) -> Result<()> {
        let unit = self.albedo.as_uniform(0)?;
        self.debug_texture.set_uniform(self.debug_uniform_in_texture, unit)?;
        self.debug_texture.draw(frame)?;
        Ok(())
    }

    pub fn debug_normal(&self, frame: &Framebuffer) -> Result<()> {
        let unit = self.normal.as_uniform(0)?;
        self.debug_texture.set_uniform(self.debug_uniform_in_texture, unit)?;
        self.debug_texture.draw(frame)?;
        Ok(())
    }

    pub fn debug_rough_metal(&self, frame: &Framebuffer) -> Result<()> {
        let unit = self.rough_metal.as_uniform(0)?;
        self.debug_texture.set_uniform(self.debug_uniform_in_texture, unit)?;
        self.debug_texture.draw(frame)?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub fn process(
        &self,
        camera: &Camera,
        lights: &LightBuffer,
    ) -> Result<&Texture<[f32;3]>> {
        self.screen_pass.set_uniform(self.uniform_camera_pos, camera.transform.position)?;
        self.output_fbo.enable_blending(Blend::One, Blend::One)?;
        self.output_fbo.clear_color([0., 0., 0., 1.])?;
        self.output_fbo.do_clear(ClearBuffer::COLOR)?;
        if lights.is_empty() {
            return Ok(&self.out_color);
        }

        let unit_pos = self.pos.as_uniform(0)?;
        let unit_albedo = self.albedo.as_uniform(1)?;
        let unit_normal = self.normal.as_uniform(2)?;
        let unit_rough_metal = self.rough_metal.as_uniform(3)?;
        self.screen_pass.set_uniform(self.uniform_frame_pos, unit_pos)?;
        self.screen_pass.set_uniform(self.uniform_frame_albedo, unit_albedo)?;
        self.screen_pass.set_uniform(self.uniform_frame_normal, unit_normal)?;
        self.screen_pass.set_uniform(self.uniform_frame_rough_metal, unit_rough_metal)?;

        for light_ix in 0..lights.len() {
            self.screen_pass
                .bind_block(self.uniform_block_light, &lights.slice(light_ix..=light_ix))?;
            self.screen_pass.draw(&self.output_fbo)?;
        }

        self.rough_metal.unbind();
        Ok(&self.out_color)
    }

    pub fn resize(&mut self, size: UVec2) -> Result<()> {
        let Some(width) = NonZeroU32::new(size.x) else { eyre::bail!("Zero width resize"); };
        let Some(height) = NonZeroU32::new(size.y) else { eyre::bail!("Zero height resize"); };
        let nonzero_one = NonZeroU32::new(1).unwrap();
        self.deferred_fbo
            .viewport(0, 0, width.get() as _, height.get() as _);
        self.output_fbo
            .viewport(0, 0, width.get() as _, height.get() as _);
        self.pos.clear_resize(width, height, nonzero_one)?;
        self.albedo.clear_resize(width, height, nonzero_one)?;
        self.normal.clear_resize(width, height, nonzero_one)?;
        self.rough_metal.clear_resize(width, height, nonzero_one)?;
        self.out_color.clear_resize(width, height, nonzero_one)?;
        self.out_depth.clear_resize(width, height, nonzero_one)?;
        Ok(())
    }
}
