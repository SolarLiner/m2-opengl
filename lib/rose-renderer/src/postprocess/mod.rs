use std::num::NonZeroU32;
use std::time::Duration;

use eyre::Result;
use glam::UVec2;

use rose_core::screen_draw::ScreenDraw;
use violette::{framebuffer::Framebuffer, program::UniformLocation, texture::Texture};
use violette::texture::{SampleMode, TextureWrap};

use crate::postprocess::autoexposure::AutoExposure;
use crate::postprocess::blur::Blur;

mod autoexposure;
mod blur;

#[derive(Debug)]
pub struct Postprocess {
    pub bloom_radius: f32,
    pub luminance_bias: f32,
    draw: ScreenDraw,
    bloom: Blur,
    auto_exposure: AutoExposure,
    u_texture: UniformLocation,
    u_avg_luminance: UniformLocation,
    texture: Texture<[f32; 3]>,
    u_bloom_tex: UniformLocation,
    u_bloom_strength: UniformLocation,
    u_lens_flare_strength: UniformLocation,
    u_lens_flare_threshold: UniformLocation,
    u_distortion_amt: UniformLocation,
    u_ghost_spacing: UniformLocation,
    u_ghost_count: UniformLocation,
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
        let draw_texture = draw.uniform("frame");
        let avg_luminance = draw.uniform("luminance_average");
        let u_bloom_strength = draw.uniform("bloom_strength");
        let u_bloom_tex = draw.uniform("bloom_tex");
        let u_lens_flare_strength = draw.uniform("lens_flare_strength");
        let u_lens_flare_threshold = draw.uniform("lens_flare_threshold");
        let u_distortion_amt = draw.uniform("distortion_amt");
        let u_ghost_spacing = draw.uniform("ghost_spacing");
        let u_ghost_count = draw.uniform("ghost_count");
        Ok(Self {
            draw,
            bloom: Blur::new(size, 5)?,
            auto_exposure: AutoExposure::new(size)?,
            u_texture: draw_texture,
            u_avg_luminance: avg_luminance,
            u_bloom_tex,
            u_bloom_strength,
            u_lens_flare_strength,
            u_lens_flare_threshold,
            u_distortion_amt,
            u_ghost_spacing,
            u_ghost_count,
            texture,
            luminance_bias: 1.5f32.exp2(),
            bloom_radius: 1e-3,
        })
    }

    pub fn set_bloom_strength(&self, strength: f32) -> Result<()> {
        self.draw
            .set_uniform(self.u_bloom_strength, strength)?;
        Ok(())
    }

    pub fn set_lens_flare_parameters(&self, params: LensFlareParams) -> Result<()> {
        self.draw.set_uniform(self.u_lens_flare_strength, params.strength)?;
        self.draw.set_uniform(self.u_lens_flare_threshold, params.threshold)?;
        self.draw.set_uniform(self.u_distortion_amt, params.distortion)?;
        self.draw.set_uniform(self.u_ghost_spacing, params.ghost_spacing)?;
        self.draw.set_uniform(self.u_ghost_count, params.ghost_count)?;
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
            self.u_avg_luminance,
            avg_luminance / self.luminance_bias,
        )?;
        let bloom = self.bloom.process(input, self.bloom_radius)?;
        self.draw
            .set_uniform(self.u_texture, input.as_uniform(0)?)?;
        self.draw
            .set_uniform(self.u_bloom_tex, bloom.as_uniform(1)?)?;
        Framebuffer::viewport(0, 0, width.get() as _, height.get() as _);
        self.draw.draw(frame)?;
        Ok(())
    }

    #[cfg(feature = "debug-ui")]
    pub fn average_luminance(&self) -> f32 {
        self.auto_exposure.average_luminance()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct LensFlareParams {
    pub strength: f32,
    pub distortion: f32,
    pub threshold: f32,
    pub ghost_spacing: f32,
    pub ghost_count: i32,
}

impl Default for LensFlareParams {
    fn default() -> Self {
        Self {
            strength: 2e-3,
            distortion: 2.,
            threshold: 1.,
            ghost_spacing: 0.31,
            ghost_count: 5,
        }
    }
}