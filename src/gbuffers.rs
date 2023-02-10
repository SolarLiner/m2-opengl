use anyhow::Context;
use winit::dpi::PhysicalSize;

use violette_low::{
    base::bindable::BindableExt,
    framebuffer::{Blend, BoundFB, ClearBuffer, Framebuffer, FramebufferFeature},
    program::{UniformBlockIndex, UniformLocation},
    texture::{DepthStencil, Dimension, SampleMode, Texture},
};

use crate::{
    camera::Camera, light::BoundLightBuffer, material::Material, mesh::Mesh,
    screen_draw::ScreenDraw,
};

macro_rules! resize_texture {
    ($size:expr, $this:ident :: $($param:ident)*) => {
        $($this.$param.bind()?.clear_resize($size.width, $size.height, 1)?);*
    };
}

macro_rules! bind_texture {
    ($($texture:expr),*) => {
        $(
        let _texture_bind = $texture.bind()?;
        )*
    };
    ($program:expr, $($location:expr , $i:literal => $texture:expr);*) => {
        $program.with_binding(|program| {
            $(
                program.set_uniform($location, $texture.as_uniform($i)?)?;
            )*
            Ok(())
        })?;
        bind_texture!($($texture),*);
    };
}

pub struct GeometryBuffers {
    screen_pass: ScreenDraw,
    debug_texture: ScreenDraw,
    fbo: Framebuffer,
    pos: Texture<[f32; 3]>,
    albedo: Texture<[f32; 3]>,
    normal: Texture<[f32; 3]>,
    rough_metal: Texture<[f32; 2]>,
    out_depth: Texture<DepthStencil<f32, ()>>,
    exposure: f32,
    uniform_exposure: UniformLocation,
    uniform_camera_pos: UniformLocation,
    uniform_frame_pos: UniformLocation,
    uniform_frame_albedo: UniformLocation,
    uniform_frame_normal: UniformLocation,
    uniform_frame_rough_metal: UniformLocation,
    uniform_block_light: UniformBlockIndex,
    debug_uniform_in_texture: UniformLocation,
}

impl GeometryBuffers {
    pub fn new(size: PhysicalSize<u32>) -> anyhow::Result<Self> {
        let mut pos = Texture::new(size.width, size.height, 1, Dimension::D2);
        pos.with_binding(|pos| {
            pos.filter_min(SampleMode::Linear)?;
            pos.filter_mag(SampleMode::Linear)?;
            pos.reserve_memory()
        })?;

        let mut albedo = Texture::new(size.width, size.height, 1, Dimension::D2);
        albedo.with_binding(|tex| {
            tex.filter_min(SampleMode::Linear)?;
            tex.filter_mag(SampleMode::Linear)?;
            tex.reserve_memory()
        })?;

        let mut normal = Texture::new(size.width, size.height, 1, Dimension::D2);
        normal.with_binding(|normal| {
            normal.filter_min(SampleMode::Linear)?;
            normal.filter_mag(SampleMode::Linear)?;
            normal.reserve_memory()
        })?;

        let mut rough_metal = Texture::new(size.width, size.height, 1, Dimension::D2);
        rough_metal.with_binding(|normal| {
            normal.filter_min(SampleMode::Linear)?;
            normal.filter_mag(SampleMode::Linear)?;
            normal.reserve_memory()
        })?;

        // let mut out_color = Texture::new(size.width, size.height, 1, Dimension::D2);
        // out_color.with_binding(|tex| {
        //     tex.filter_min(SampleMode::Linear)?;
        //     tex.filter_mag(SampleMode::Linear)?;
        //     tex.reserve_memory()
        // })?;

        let mut out_depth = Texture::new(size.width, size.height, 1, Dimension::D2);
        out_depth.with_binding(|tex| {
            tex.filter_min(SampleMode::Linear)?;
            tex.filter_mag(SampleMode::Linear)?;
            tex.reserve_memory()
        })?;

        let mut fbo = Framebuffer::new();
        fbo.with_binding(|fbo| {
            fbo.attach_color(0, &pos)?;
            fbo.attach_color(1, &albedo)?;
            fbo.attach_color(2, &normal)?;
            fbo.attach_color(3, &rough_metal)?;
            fbo.attach_depth(&out_depth)?;
            fbo.assert_complete()
        })?;

        let mut screen_pass = ScreenDraw::load("assets/shaders/defferred.frag.glsl")
            .context("Cannot load screen shader pass")?;
        let debug_texture = ScreenDraw::load("assets/shaders/blit.frag.glsl")
            .context("Cannot load blit program")?;
        let debug_uniform_in_texture = debug_texture.uniform("in_texture").unwrap();

        let uniform_exposure = screen_pass.uniform("exposure").unwrap();
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
            // out_color,
            out_depth,
            uniform_exposure,
            uniform_camera_pos,
            debug_uniform_in_texture,
            uniform_frame_pos,
            uniform_frame_albedo,
            uniform_frame_normal,
            uniform_frame_rough_metal,
            uniform_block_light,
            screen_pass,
            debug_texture,
            exposure: 1.,
        })
    }

    pub fn set_exposure(&mut self, v: f32) {
        self.exposure = v;
    }

    pub fn framebuffer(&mut self) -> &mut Framebuffer {
        &mut self.fbo
    }

    #[tracing::instrument(skip_all)]
    pub fn draw_meshes(
        &mut self,
        camera: &Camera,
        material: &mut Material,
        meshes: &mut [Mesh],
    ) -> anyhow::Result<()> {
        let mut fbo = self.fbo.bind()?;
        fbo.enable_buffers([0, 1, 2, 3])?;
        bind_texture!(self.pos, self.albedo, self.normal, self.rough_metal);
        material.draw_meshes(&mut fbo, camera, meshes)?;

        Ok(())
    }

    pub fn debug_position(&mut self, frame: &mut BoundFB) -> anyhow::Result<()> {
        let (_bind, unit) = self.pos.as_uniform(0)?;
        self.debug_texture.bind()?.set_uniform(self.debug_uniform_in_texture, unit)?;
        self.debug_texture.draw(frame)
    }

    pub fn debug_albedo(&mut self, frame: &mut BoundFB) -> anyhow::Result<()> {
        let (_bind, unit) = self.albedo.as_uniform(0)?;
        self.debug_texture.bind()?.set_uniform(self.debug_uniform_in_texture, unit)?;
        self.debug_texture.draw(frame)
    }

    pub fn debug_normal(&mut self, frame: &mut BoundFB) -> anyhow::Result<()> {
        let (_bind, unit) = self.normal.as_uniform(0)?;
        self.debug_texture.bind()?.set_uniform(self.debug_uniform_in_texture, unit)?;
        self.debug_texture.draw(frame)
    }

    pub fn debug_rough_metal(&mut self, frame: &mut BoundFB) -> anyhow::Result<()> {
        let (_bind, unit) = self.normal.as_uniform(0)?;
        self.debug_texture.bind()?.set_uniform(self.debug_uniform_in_texture, unit)?;
        self.debug_texture.draw(frame)
    }

    #[tracing::instrument(skip_all)]
    pub fn draw_screen(
        &mut self,
        frame: &mut BoundFB,
        camera: &Camera,
        lights: &mut BoundLightBuffer,
    ) -> anyhow::Result<()> {
        self.screen_pass.with_binding(|screen_program| {
            screen_program.set_uniform(self.uniform_exposure, self.exposure)?;
            screen_program.set_uniform(self.uniform_camera_pos, camera.transform.position)?;
            Ok(())
        })?;

        frame.set_feature(FramebufferFeature::Blending(Blend::SrcAlpha, Blend::One))?; // Additive blending
        frame.do_clear(ClearBuffer::COLOR)?;
        if lights.is_empty() {
            return Ok(());
        }

        let (_bind, unit_pos) = self.pos.as_uniform(0)?;
        let (_bind, unit_albedo) = self.albedo.as_uniform(1)?;
        let (_bind, unit_normal) = self.normal.as_uniform(2)?;
        let (_bind, unit_rough_metal) = self.rough_metal.as_uniform(3)?;
        self.screen_pass.with_binding(|prog| {
            prog.set_uniform(self.uniform_frame_pos, unit_pos)?;
            prog.set_uniform(self.uniform_frame_albedo, unit_albedo)?;
            prog.set_uniform(self.uniform_frame_normal, unit_normal)?;
            prog.set_uniform(self.uniform_frame_rough_metal, unit_rough_metal)?;
            Ok(())
        })?;
        for light_ix in 0..lights.len() {
            self.screen_pass
                .bind()?
                .bind_block(self.uniform_block_light, &lights.slice(light_ix..=light_ix))?;
            self.screen_pass.draw(frame)?;
        }
        Ok(())
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) -> anyhow::Result<()> {
        self.fbo
            .bind()?
            .viewport(0, 0, size.width as _, size.height as _);
        resize_texture!(size, self::pos albedo normal rough_metal /* out_color */ out_depth);
        Ok(())
    }
}
