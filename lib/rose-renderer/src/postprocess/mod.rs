use std::num::NonZeroU32;
use std::time::Duration;

use eyre::Result;
use glam::UVec2;

use rose_core::screen_draw::ScreenDraw;
use violette::texture::{SampleMode, TextureWrap};
use violette::{framebuffer::Framebuffer, program::UniformLocation, texture::Texture};

use crate::postprocess::autoexposure::AutoExposure;

mod autoexposure;

#[derive(Debug)]
pub struct Postprocess {
    draw: ScreenDraw,
    auto_exposure: AutoExposure,
    uniform_texture: UniformLocation,
    uniform_avg_luminance: UniformLocation,
    texture: Texture<[f32; 3]>,
    uniform_bloom_strength: UniformLocation,
    uniform_bloom_size: UniformLocation,
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
        let draw_bloom_strength = draw.uniform("bloom_strength").unwrap();
        let draw_bloom_size = draw.uniform("bloom_size").unwrap();
        Ok(Self {
            draw,
            auto_exposure: AutoExposure::new(size)?,
            uniform_texture: draw_texture,
            uniform_avg_luminance: avg_luminance,
            uniform_bloom_size: draw_bloom_size,
            uniform_bloom_strength: draw_bloom_strength,
            texture,
            luminance_bias: 1.,
        })
    }

    pub fn set_bloom_strength(&self, strength: f32) -> Result<()> {
        self.draw
            .set_uniform(self.uniform_bloom_strength, strength)?;
        Ok(())
    }

    pub fn set_bloom_size(&self, size: f32) -> Result<()> {
        self.draw.set_uniform(self.uniform_bloom_size, size)?;
        Ok(())
    }

    pub fn resize(&mut self, size: UVec2) -> Result<()> {
        let Some(width) = NonZeroU32::new(size.x) else { eyre::bail!("Zero width resize"); };
        let Some(height) = NonZeroU32::new(size.y) else { eyre::bail!("Zero height resize"); };
        self.texture
            .clear_resize(width, height, NonZeroU32::new(1).unwrap())?;
        self.auto_exposure.resize(size)?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub fn draw(&mut self, frame: &Framebuffer, input: &Texture<[f32; 3]>, dt: Duration) -> Result<()> {
        let accomodate = dt.as_secs_f32() * 100.;
        let lerp = accomodate / (1. + accomodate);
        tracing::debug!(?accomodate, ?lerp);
        let avg_luminance = self.auto_exposure.process(input, lerp)?;
        self.draw.set_uniform(
            self.uniform_avg_luminance,
            avg_luminance / self.luminance_bias,
        )?;
        self.draw
            .set_uniform(self.uniform_texture, input.as_uniform(0)?)?;
        self.draw.draw(frame)?;
        Ok(())
    }

    #[cfg(feature = "debug-ui")]
    pub fn average_luminance(&self) -> f32 {
        self.auto_exposure.average_luminance()
    }
}
