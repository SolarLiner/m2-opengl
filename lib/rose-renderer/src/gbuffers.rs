use std::num::NonZeroU32;

use eyre::{Context, Result};
use glam::UVec2;

use rose_core::{
    camera::ViewUniformBuffer,
    light::LightBuffer,
    screen_draw::ScreenDraw,
    transform::Transformed,
    utils::reload_watcher::ReloadWatcher,
};
use violette::{
    base::resource::Resource,
    framebuffer::{Blend, ClearBuffer, DepthTestFunction, Framebuffer},
    program::{UniformBlockIndex, UniformLocation},
    texture::{DepthStencil, Dimension, SampleMode, Texture},
};

use crate::{
    env::{Environment, MaterialInfo},
    material::{Material, MaterialInstance},
    Mesh,
};

#[derive(Debug)]
pub struct GeometryBuffers {
    screen_pass: ScreenDraw,
    blit: ScreenDraw,
    deferred_fbo: Framebuffer,
    output_fbo: Framebuffer,
    size: UVec2,
    pos: Texture<[f32; 3]>,
    albedo: Texture<[f32; 3]>,
    normal_coverage: Texture<[f32; 4]>,
    rough_metal: Texture<[f32; 2]>,
    emission: Texture<[f32; 3]>,
    out_color: Texture<[f32; 3]>,
    out_depth: Texture<DepthStencil<f32, ()>>,
    uniform_frame_pos: UniformLocation,
    uniform_frame_albedo: UniformLocation,
    uniform_frame_normal: UniformLocation,
    uniform_frame_rough_metal: UniformLocation,
    uniform_frame_emission: UniformLocation,
    uniform_block_light: UniformBlockIndex,
    uniform_block_view: UniformBlockIndex,
    uniform_blit_source: UniformLocation,
}

impl GeometryBuffers {
    pub fn new(size: UVec2, reload_watcher: &ReloadWatcher) -> Result<Self> {
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

        let normal_coverage = Texture::new(width, height, nonzero_one, Dimension::D2);
        normal_coverage.filter_min(SampleMode::Linear)?;
        normal_coverage.filter_mag(SampleMode::Linear)?;
        normal_coverage.reserve_memory()?;

        let rough_metal = Texture::new(width, height, nonzero_one, Dimension::D2);
        rough_metal.filter_min(SampleMode::Linear)?;
        rough_metal.filter_mag(SampleMode::Linear)?;
        rough_metal.reserve_memory()?;

        let emission = Texture::new(width, height, nonzero_one, Dimension::D2);
        emission.filter_min(SampleMode::Linear)?;
        emission.filter_mag(SampleMode::Linear)?;
        emission.reserve_memory()?;

        let out_color = Texture::new(width, height, nonzero_one, Dimension::D2);
        out_color.filter_min(SampleMode::Linear)?;
        out_color.filter_mag(SampleMode::Linear)?;
        out_color.reserve_memory()?;

        let out_depth = Texture::new(width, height, nonzero_one, Dimension::D2);
        out_depth.filter_min(SampleMode::Linear)?;
        out_depth.filter_mag(SampleMode::Linear)?;
        out_depth.reserve_memory()?;

        let deferred_fbo = Framebuffer::new();
        deferred_fbo.attach_color(0, pos.mipmap(0).unwrap())?;
        deferred_fbo.attach_color(1, albedo.mipmap(0).unwrap())?;
        deferred_fbo.attach_color(2, normal_coverage.mipmap(0).unwrap())?;
        deferred_fbo.attach_color(3, rough_metal.mipmap(0).unwrap())?;
        deferred_fbo.attach_color(4, emission.mipmap(0).unwrap())?;
        deferred_fbo.attach_depth(&out_depth)?;
        deferred_fbo.enable_buffers([0, 1, 2, 3, 4])?;
        deferred_fbo.assert_complete()?;

        let output_fbo = Framebuffer::new();
        output_fbo.attach_color(0, out_color.mipmap(0).unwrap())?;
        output_fbo.assert_complete()?;

        let screen_pass = ScreenDraw::load("screen/deferred.glsl", reload_watcher)
            .context("Cannot load screen shader pass")?;
        let blit = ScreenDraw::load("blit.glsl", reload_watcher)
            .context("Cannot load blit program")?;
        let debug_uniform_in_texture = blit.program().uniform("in_texture");

        let pass_program = screen_pass.program();
        let uniform_frame_pos = pass_program.uniform("frame_position");
        let uniform_frame_albedo = pass_program.uniform("frame_albedo");
        let uniform_frame_normal = pass_program.uniform("frame_normal");
        let uniform_frame_rough_metal = pass_program.uniform("frame_rough_metal");
        let uniform_frame_emission = pass_program.uniform("frame_emission");
        let uniform_block_light = pass_program.uniform_block("Light");
        let uniform_block_view = pass_program.uniform_block("View");
        drop(pass_program);

        Ok(Self {
            deferred_fbo,
            output_fbo,
            size,
            pos,
            albedo,
            normal_coverage,
            rough_metal,
            emission,
            out_color,
            out_depth,
            uniform_blit_source: debug_uniform_in_texture,
            uniform_frame_pos,
            uniform_frame_albedo,
            uniform_frame_normal,
            uniform_frame_rough_metal,
            uniform_frame_emission,
            uniform_block_light,
            uniform_block_view,
            screen_pass,
            blit,
        })
    }

    pub fn framebuffer(&self) -> &Framebuffer {
        &self.deferred_fbo
    }

    #[tracing::instrument(skip_all)]
    pub fn draw_meshes<MC: std::ops::Deref<Target=Mesh>>(
        &self,
        material: &mut Material,
        instance: &MaterialInstance,
        meshes: &[Transformed<MC>],
    ) -> Result<()> {
        Framebuffer::disable_blending();
        Framebuffer::disable_scissor();
        Framebuffer::enable_depth_test(DepthTestFunction::Less);
        material.draw_meshes(&self.deferred_fbo, instance, meshes)?;

        Ok(())
    }

    pub fn debug_position(&self, frame: &Framebuffer) -> Result<()> {
        let unit = self.pos.as_uniform(0)?;
        self.blit
            .program()
            .set_uniform(self.uniform_blit_source, unit)?;
        self.blit.draw(frame)?;
        Ok(())
    }

    pub fn debug_albedo(&self, frame: &Framebuffer) -> Result<()> {
        let unit = self.albedo.as_uniform(0)?;
        self.blit
            .program()
            .set_uniform(self.uniform_blit_source, unit)?;
        self.blit.draw(frame)?;
        Ok(())
    }

    pub fn debug_normal(&self, frame: &Framebuffer) -> Result<()> {
        let unit = self.normal_coverage.as_uniform(0)?;
        self.blit
            .program()
            .set_uniform(self.uniform_blit_source, unit)?;
        self.blit.draw(frame)?;
        Ok(())
    }

    pub fn debug_rough_metal(&self, frame: &Framebuffer) -> Result<()> {
        let unit = self.rough_metal.as_uniform(0)?;
        self.blit
            .program()
            .set_uniform(self.uniform_blit_source, unit)?;
        self.blit.draw(frame)?;
        Ok(())
    }

    pub fn debug_emission(&self, frame: &Framebuffer) -> Result<()> {
        let unit = self.emission.as_uniform(0)?;
        self.blit
            .program()
            .set_uniform(self.uniform_blit_source, unit)?;
        self.blit.draw(frame)?;
        Ok(())
    }


    pub fn size(&self) -> UVec2 {
        self.size
    }

    #[tracing::instrument(skip_all)]
    pub fn process(
        &self,
        cam_uniform: &ViewUniformBuffer,
        lights: &LightBuffer,
        mut env: Option<&mut dyn Environment>,
    ) -> Result<&Texture<[f32; 3]>> {
        Framebuffer::enable_blending(Blend::One, Blend::One);
        Framebuffer::clear_color([0., 0., 0., 1.]);
        self.output_fbo.do_clear(ClearBuffer::COLOR);

        {
            let program = self.blit.program();
            program.set_uniform(self.uniform_blit_source, self.emission.as_uniform(3)?)?;
        }
        self.blit.draw(&self.output_fbo)?;
        if let Some(env) = &mut env {
            let mat_info = MaterialInfo {
                position: &self.pos,
                albedo: &self.albedo,
                normal_coverage: &self.normal_coverage,
                roughness_metal: &self.rough_metal,
            };
            env.draw(&self.output_fbo, cam_uniform, mat_info)?;
        }

        if lights.is_empty() {
            return Ok(&self.out_color);
        }

        let unit_pos = self.pos.as_uniform(0)?;
        let unit_albedo = self.albedo.as_uniform(1)?;
        let unit_normal = self.normal_coverage.as_uniform(2)?;
        let unit_rough_metal = self.rough_metal.as_uniform(3)?;
        let unit_emission = self.emission.as_uniform(3)?;
        {
            let pass_program = self.screen_pass.program();
            pass_program.set_uniform(self.uniform_frame_pos, unit_pos)?;
            pass_program.set_uniform(self.uniform_frame_albedo, unit_albedo)?;
            pass_program.set_uniform(self.uniform_frame_normal, unit_normal)?;
            pass_program.set_uniform(self.uniform_frame_rough_metal, unit_rough_metal)?;
            pass_program.set_uniform(self.uniform_frame_emission, unit_emission)?;
        }

        for light_ix in 0..lights.len() {
            self.screen_pass.program().bind_block(
                &lights.slice(light_ix..=light_ix),
                self.uniform_block_light,
                0,
            )?;
            self.screen_pass.draw(&self.output_fbo)?;
        }

        self.rough_metal.unbind();
        Ok(&self.out_color)
    }

    pub fn resize(&mut self, size: UVec2) -> Result<()> {
        let Some(width) = NonZeroU32::new(size.x) else { eyre::bail!("Zero width resize"); };
        let Some(height) = NonZeroU32::new(size.y) else { eyre::bail!("Zero height resize"); };
        let nonzero_one = NonZeroU32::new(1).unwrap();
        self.pos.clear_resize(width, height, nonzero_one)?;
        self.albedo.clear_resize(width, height, nonzero_one)?;
        self.normal_coverage
            .clear_resize(width, height, nonzero_one)?;
        self.rough_metal.clear_resize(width, height, nonzero_one)?;
        self.emission.clear_resize(width, height, nonzero_one)?;
        self.out_color.clear_resize(width, height, nonzero_one)?;
        self.out_depth.clear_resize(width, height, nonzero_one)?;
        Ok(())
    }
}
