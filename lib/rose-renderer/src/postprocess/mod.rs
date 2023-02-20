use std::num::NonZeroU32;
use std::time::Duration;

use eyre::Result;
use glam::UVec2;

use rose_core::screen_draw::ScreenDraw;
use violette::texture::{SampleMode, TextureWrap};
use violette::{framebuffer::Framebuffer, program::UniformLocation, texture::Texture};

use crate::postprocess::autoexposure::AutoExposure;
use crate::postprocess::blur::Blur;

mod autoexposure;
mod blur;

#[derive(Debug)]
pub struct Postprocess {
    draw: ScreenDraw,
    bloom: Blur,
    auto_exposure: AutoExposure,
    uniform_texture: UniformLocation,
    uniform_avg_luminance: UniformLocation,
    texture: Texture<[f32; 3]>,
    uniform_bloom_tex: UniformLocation,
    uniform_bloom_strength: UniformLocation,
    pub bloom_radius: f32,
    pub luminance_bias: f32,
}

impl Postprocess {
    pub fn new(size: UVec2) -> Result<Self> {
        let Some(width) = NonZeroU32::new(size.x) else { eyre::bail!("Zero width resize"); };
        let Some(height) = NonZeroU32::new(size.y) else { eyre::bail!("Zero height resize"); };
        let nonzero_one = NonZeroU32::new(1).unwrap();
        let texture = Texture::new(width, height, nonzero_one, violette::texture::Dimension::D2);
        texture.wrap_r(TextureWrap::MirroredRepeat)?;
        texture.wrap_s(TextureWrap::MirroredRepeat)?;
        texture.filter_min(SampleMode::Nearest)?;
        texture.filter_mag(SampleMode::Linear)?;
        texture.reserve_memory()?;

        let draw = ScreenDraw::load("assets/shaders/postprocess.frag.glsl")?;
        let draw_texture = draw.uniform("frame").unwrap();
        let avg_luminance = draw.uniform("luminance_average").unwrap();
        let uniform_bloom_strength = draw.uniform("bloom_strength").unwrap();
        let uniform_bloom_tex = draw.uniform("bloom_tex").unwrap();
        Ok(Self {
            draw,
            bloom: Blur::new(size, 10)?,
            auto_exposure: AutoExposure::new(size)?,
            uniform_texture: draw_texture,
            uniform_avg_luminance: avg_luminance,
            uniform_bloom_tex,
            uniform_bloom_strength,
            texture,
            luminance_bias: 1.,
            bloom_radius: 10.,
        })
    }

    pub fn set_bloom_strength(&self, strength: f32) -> Result<()> {
        self.draw
            .set_uniform(self.uniform_bloom_strength, strength)?;
        Ok(())
    }

    pub fn resize(&mut self, size: UVec2) -> Result<()> {
        let Some(width) = NonZeroU32::new(size.x) else { eyre::bail!("Zero width resize"); };
        let Some(height) = NonZeroU32::new(size.y) else { eyre::bail!("Zero height resize"); };
        self.texture
            .clear_resize(width, height, NonZeroU32::new(1).unwrap())?;
        self.auto_exposure.resize(size)?;
        self.bloom.resize(width, height)?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub fn draw(
        &mut self,
        frame: &Framebuffer,
        input: &Texture<[f32; 3]>,
        dt: Duration,
    ) -> Result<()> {
        let (width, height) = input.mipmap_size(0).unwrap();
        let accomodate = dt.as_secs_f32() * 100.;
        let lerp = accomodate / (1. + accomodate);
        tracing::debug!(?accomodate, ?lerp);
        let avg_luminance = self
            .auto_exposure
            .process(input, lerp)
            .unwrap_or_else(|_| self.auto_exposure.average_luminance());
        self.draw.set_uniform(
            self.uniform_avg_luminance,
            avg_luminance / self.luminance_bias,
        )?;
        let bloom = self.bloom.process(input, self.bloom_radius)?;
        self.draw
            .set_uniform(self.uniform_texture, input.as_uniform(0)?)?;
        self.draw
            .set_uniform(self.uniform_bloom_tex, bloom.as_uniform(1)?)?;
        frame.viewport(0, 0, width.get() as _, height.get() as _);
        self.draw.draw(frame)?;
        Ok(())
    }

    #[cfg(feature = "debug-ui")]
    pub fn average_luminance(&self) -> f32 {
        self.auto_exposure.average_luminance()
    }
}
