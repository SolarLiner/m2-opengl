use std::num::NonZeroU32;
use std::time::Duration;

use eyre::Result;
use glam::UVec2;

use rose_core::screen_draw::ScreenDraw;
use violette::program::UniformLocation;
use violette::{
    framebuffer::Framebuffer,
    texture::{Dimension, SampleMode, Texture},
};
use violette::framebuffer::ClearBuffer;

#[derive(Debug)]
pub struct AutoExposure {
    screen_draw: ScreenDraw,
    uniform_in_texture: UniformLocation,
    fbo: Framebuffer,
    target: Texture<f32>,
    avg_luminance: f32,
}

impl AutoExposure {
    pub fn new(size: UVec2) -> Result<Self> {
        let Some(width) = NonZeroU32::new(size.x) else {
            eyre::bail!("Non zero size");
        };
        let Some(height) = NonZeroU32::new(size.y) else {
            eyre::bail!("Non zero size");
        };
        let depth = unsafe { NonZeroU32::new_unchecked(1) };
        let screen_draw = ScreenDraw::load("assets/shaders/luminance-estimate.frag.glsl")?;
        let target = Texture::new(width, height, depth, Dimension::D2);
        target.filter_mag(SampleMode::Linear)?;
        target.filter_min(SampleMode::Linear)?;
        target.reserve_memory()?;
        let fbo = Framebuffer::new();
        fbo.viewport(0, 0, size.x as _, size.y as _);
        fbo.disable_blending()?;
        fbo.disable_depth_test()?;
        fbo.clear_color([0., 0., 0., 1.])?;
        fbo.attach_color(0, &target)?;
        fbo.assert_complete()?;
        let uniform_in_texture = screen_draw.uniform("in_texture").unwrap();
        Ok(Self {
            screen_draw,
            uniform_in_texture,
            fbo,
            target,
            avg_luminance: 0.5,
        })
    }

    pub fn resize(&mut self, size: UVec2) -> Result<()> {
        let Some(width) = NonZeroU32::new(size.x) else {
            eyre::bail!("Non zero size");
        };
        let Some(height) = NonZeroU32::new(size.y) else {
            eyre::bail!("Non zero size");
        };
        let depth = unsafe { NonZeroU32::new_unchecked(1) };
        self.target.clear_resize(width, height, depth)?;
        self.fbo.viewport(0, 0, size.x as _, size.y as _);
        Ok(())
    }

    pub fn average_luminance(&self) -> f32 {
        self.avg_luminance
    }

    #[tracing::instrument(skip_all)]
    pub fn process(&mut self, in_texture: &Texture<[f32; 3]>, lerp: f32) -> Result<f32> {
        self.screen_draw
            .set_uniform(self.uniform_in_texture, in_texture.as_uniform(0)?)?;
        self.fbo.do_clear(ClearBuffer::COLOR)?;
        self.screen_draw.draw(&self.fbo)?;
        self.target.generate_mipmaps()?;
        let luminance_data = self.target.download(self.target.num_mipmaps() - 1)?;
        let luminance_data = luminance_data[0].max(1e-32);
        self.avg_luminance += (luminance_data - self.avg_luminance) * lerp;
        tracing::debug!(avg_luminance=?self.avg_luminance, luminance=?luminance_data);
        Ok(self.avg_luminance)
    }
}
