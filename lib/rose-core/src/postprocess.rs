use std::{num::NonZeroU32};
use glam::UVec2;

use violette::{framebuffer::Framebuffer, texture::Texture, program::UniformLocation};
use eyre::Result;

use crate::screen_draw::ScreenDraw;

pub struct Postprocess {
    draw: ScreenDraw,
    draw_texture: UniformLocation,
    draw_exposure: UniformLocation,
    fbo: Framebuffer,
    texture: Texture<[f32; 3]>,
}

impl Postprocess {
    pub fn new(size: UVec2) -> Result<Self> {
        let Some(width) = NonZeroU32::new(size.x) else { eyre::bail!("Zero width resize"); };
        let Some(height) = NonZeroU32::new(size.y) else { eyre::bail!("Zero height resize"); };
        let nonzero_one = NonZeroU32::new(1).unwrap();
        let fbo = Framebuffer::new();
        let texture = Texture::new(width, height, nonzero_one, violette::texture::Dimension::D2);
        texture.wrap_r(violette::texture::TextureWrap::MirroredRepeat)?;
        texture.wrap_s(violette::texture::TextureWrap::MirroredRepeat)?;
        texture.filter_min(violette::texture::SampleMode::Nearest)?;
        texture.filter_mag(violette::texture::SampleMode::Linear)?;
        texture.reserve_memory()?;
        fbo.attach_color(0, &texture)?;
        fbo.assert_complete()?;
        fbo.viewport(0, 0, size.x as _, size.y as _);

        let draw = ScreenDraw::load("assets/shaders/postprocess.frag.glsl")?;
        let draw_texture = draw.uniform("frame").unwrap();
        let draw_exposure = draw.uniform("exposure").unwrap();
        Ok(Self { draw, draw_texture, draw_exposure, fbo, texture })
    }

    pub fn framebuffer(&self) -> &Framebuffer {
        &self.fbo
    }

    pub fn set_exposure(&self, exposure: f32) -> Result<()> {
        self.draw.set_uniform(self.draw_exposure, exposure)?;
        Ok(())
    }

    pub fn resize(&mut self, size: UVec2) -> Result<()> {
        self.fbo.viewport(0, 0, size.x as _, size.y as _);
        let Some(width) = NonZeroU32::new(size.x) else { eyre::bail!("Zero width resize"); };
        let Some(height) = NonZeroU32::new(size.y) else { eyre::bail!("Zero height resize"); };
        self.texture.clear_resize(width, height, NonZeroU32::new(1).unwrap())?;
        Ok(())
    }

    pub fn draw(&mut self, frame: &Framebuffer) -> Result<()> {
        self.draw.set_uniform(self.draw_texture, self.texture.as_uniform(0)?)?;
        self.draw.draw(frame)?;
        Ok(())
    }
}