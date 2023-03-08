use std::num::NonZeroU32;

use eyre::Result;
use glam::{UVec2, Vec3};

use rose_core::screen_draw::ScreenDraw;
use violette::{
    framebuffer::Framebuffer,
    texture::{Dimension, SampleMode, Texture},
};
use violette::program::UniformLocation;

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
        fbo.attach_color(0, &target)?;
        fbo.assert_complete()?;
        let uniform_in_texture = screen_draw.uniform("in_texture");
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
        Ok(())
    }

    pub fn average_luminance(&self) -> f32 {
        self.avg_luminance
    }

    #[tracing::instrument(skip_all)]
    pub fn process(&mut self, in_texture: &Texture<[f32; 3]>, lerp: f32) -> Result<f32> {
        self.screen_draw
            .set_uniform(self.uniform_in_texture, in_texture.as_uniform(0)?)?;
        Framebuffer::disable_blending();
        Framebuffer::disable_depth_test();
        Framebuffer::clear_color(Vec3::ZERO.extend(1.).to_array());
        Framebuffer::disable_blending();
        let (width, height, _) = in_texture.size();
        Framebuffer::viewport(0, 0, width.get() as _, height.get() as _);
        self.screen_draw.draw(&self.fbo)?;
        self.target.generate_mipmaps()?;
        let last_mipmap = self.target.num_mipmaps() - 1;
        tracing::debug!(message="Sampling last mipmap for average", mipmap=%last_mipmap);
        let luminance_data = self.target.download(last_mipmap)?;
        let mut luminance_data = luminance_data[0];
        if luminance_data.is_nan() {
            luminance_data = 1.;
        }
        tracing::debug!(%luminance_data, ev=%luminance_data.log2());
        self.avg_luminance += (luminance_data - self.avg_luminance) * lerp;
        tracing::debug!(avg_luminance=?self.avg_luminance, luminance=?luminance_data);
        Ok(self.avg_luminance)
    }
}
