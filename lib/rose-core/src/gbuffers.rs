use std::num::NonZeroU32;
use glam::UVec2;

use eyre::{Context, Result};

use violette::{
    base::resource::{Resource},
    framebuffer::{
        Blend,
        ClearBuffer,
        Framebuffer,
        DepthTestFunction,
    },
    program::{UniformBlockIndex, UniformLocation},
    texture::{DepthStencil, Dimension, SampleMode, Texture},
};

use crate::{
    camera::Camera, material::{Material, Vertex}, mesh::Mesh,
    screen_draw::ScreenDraw,
};
use crate::light::LightBuffer;

pub struct GeometryBuffers {
    screen_pass: ScreenDraw,
    debug_texture: ScreenDraw,
    fbo: Framebuffer,
    pos: Texture<[f32; 3]>,
    albedo: Texture<[f32; 3]>,
    normal: Texture<[f32; 3]>,
    rough_metal: Texture<[f32; 2]>,
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

        let out_depth = Texture::new(width, height, nonzero_one, Dimension::D2);
        out_depth.filter_min(SampleMode::Linear)?;
        out_depth.filter_mag(SampleMode::Linear)?;
        out_depth.reserve_memory()?;

        let fbo = Framebuffer::new();
        fbo.attach_color(0, &pos)?;
        fbo.attach_color(1, &albedo)?;
        fbo.attach_color(2, &normal)?;
        fbo.attach_color(3, &rough_metal)?;
        fbo.attach_depth(&out_depth)?;
        fbo.assert_complete()?;
        fbo.clear_color([0., 0., 0., 1.])?;
        fbo.clear_depth(1.)?;
        fbo.viewport(0, 0, size.x as _, size.y as _);

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
            fbo,
            pos,
            albedo,
            normal,
            rough_metal,
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
        &self.fbo
    }

    #[tracing::instrument(skip_all)]
    pub fn draw_meshes(
        &self,
        camera: &Camera,
        material: &Material,
        meshes: &mut [Mesh<Vertex>],
    ) -> Result<()> {
        self.fbo.viewport(0, 0, camera.projection.width as _, camera.projection.height as _);
        self.fbo.disable_blending()?;
        self.fbo.disable_scissor()?;
        self.fbo.enable_depth_test(DepthTestFunction::Less)?;
        self.fbo.enable_buffers([0, 1, 2, 3])?;
        self.fbo.do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH)?;
        // self.pos.with_binding(|| self.albedo.with_binding(|| self.normal.with_binding(|| self.rough_metal.with_binding(|| material.draw_meshes(&self.fbo, camera, meshes)))))?;
        material.draw_meshes(&self.fbo, camera, meshes)?;

        Ok(())
    }

    pub fn debug_position(&mut self, frame: &Framebuffer) -> Result<()> {
        let unit = self.pos.as_uniform(0)?;
        self.debug_texture.set_uniform(self.debug_uniform_in_texture, unit)?;
        self.debug_texture.draw(frame)?;
        Ok(())
    }

    pub fn debug_albedo(&mut self, frame: &Framebuffer) -> Result<()> {
        let unit = self.albedo.as_uniform(0)?;
        self.debug_texture.set_uniform(self.debug_uniform_in_texture, unit)?;
        self.debug_texture.draw(frame)?;
        Ok(())
    }

    pub fn debug_normal(&mut self, frame: &Framebuffer) -> Result<()> {
        let unit = self.normal.as_uniform(0)?;
        self.debug_texture.set_uniform(self.debug_uniform_in_texture, unit)?;
        self.debug_texture.draw(frame)?;
        Ok(())
    }

    pub fn debug_rough_metal(&mut self, frame: &Framebuffer) -> Result<()> {
        let unit = self.rough_metal.as_uniform(0)?;
        self.debug_texture.set_uniform(self.debug_uniform_in_texture, unit)?;
        self.debug_texture.draw(frame)?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub fn draw_screen(
        &mut self,
        frame: &Framebuffer,
        camera: &Camera,
        lights: &LightBuffer,
    ) -> Result<()> {
        self.screen_pass.set_uniform(self.uniform_camera_pos, camera.transform.position)?;

        frame.disable_depth_test()?;
        frame.enable_blending(Blend::One, Blend::One)?;
        frame.do_clear(ClearBuffer::COLOR)?;
        if lights.is_empty() {
            return Ok(());
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
            self.screen_pass.draw(frame)?;
        }

        self.rough_metal.unbind();
        Ok(())
    }

    pub fn resize(&mut self, size: UVec2) -> Result<()> {
        let Some(width) = NonZeroU32::new(size.x) else { eyre::bail!("Zero width resize"); };
        let Some(height) = NonZeroU32::new(size.y) else { eyre::bail!("Zero height resize"); };
        let nonzero_one = NonZeroU32::new(1).unwrap();
        self.fbo
            .viewport(0, 0, width.get() as _, height.get() as _);
        self.pos.clear_resize(width, height, nonzero_one)?;
        self.albedo.clear_resize(width, height, nonzero_one)?;
        self.normal.clear_resize(width, height, nonzero_one)?;
        self.rough_metal.clear_resize(width, height, nonzero_one)?;
        self.out_depth.clear_resize(width, height, nonzero_one)?;
        Ok(())
    }
}
