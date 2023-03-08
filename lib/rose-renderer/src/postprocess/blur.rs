use std::num::NonZeroU32;

use eyre::Result;
use glam::UVec2;

use rose_core::screen_draw::ScreenDraw;
use violette::{
    framebuffer::{Blend, BlendFunction, Framebuffer},
    program::UniformLocation,
    texture::{Dimension, SampleMode, Texture, TextureWrap},
};

#[derive(Debug)]
pub struct Blur {
    mip_chain: Vec<Texture<[f32; 3]>>,
    fbo: Framebuffer,
    draw_downsample: ScreenDraw,
    draw_upsample: ScreenDraw,
    uniform_down_tex: UniformLocation,
    uniform_down_size: UniformLocation,
    uniform_up_tex: UniformLocation,
    uniform_up_radius: UniformLocation,
}

impl Blur {
    pub fn new(size: UVec2, mip_chain_len: usize) -> Result<Self> {
        let sizef = size.as_vec2();
        let max_chain_len = sizef.x.min(sizef.y).log2().floor() as usize - 1;
        eyre::ensure!(mip_chain_len <= max_chain_len, "Cannot construct a chain longer than {} for  {}", max_chain_len, size);
        let Some(width) = NonZeroU32::new(size.x) else { eyre::bail!("Zero size"); };
        let Some(height) = NonZeroU32::new(size.x) else { eyre::bail!("Zero size"); };
        let depth = NonZeroU32::new(1).unwrap();

        let mip_chain = (0..mip_chain_len).try_fold(vec![], |mut vec, _| {
            let mip = Texture::new(depth, depth, depth, Dimension::D2);
            mip.filter_min(SampleMode::Linear)?;
            mip.filter_mag(SampleMode::Linear)?;
            mip.wrap_r(TextureWrap::MirroredRepeat)?;
            mip.wrap_s(TextureWrap::MirroredRepeat)?;
            mip.wrap_t(TextureWrap::MirroredRepeat)?;
            mip.reserve_memory()?;
            vec.push(mip);
            Ok::<_, eyre::Report>(vec)
        })?;

        let draw_downsample = ScreenDraw::load("assets/shaders/blur_downsample.frag.glsl")?;
        let draw_upsample = ScreenDraw::load("assets/shaders/blur_upsample.frag.glsl")?;
        let fbo = Framebuffer::new();
        fbo.attach_color(0, &mip_chain[0])?;
        fbo.enable_buffers([0])?;
        fbo.assert_complete()?;

        let uniform_down_tex = draw_downsample.uniform("in_texture");
        let uniform_down_size = draw_downsample.uniform("screen_size");
        let uniform_up_tex = draw_upsample.uniform("in_texture");
        let uniform_up_radius = draw_upsample.uniform("filter_radius");
        let mut this = Self {
            mip_chain,
            fbo,
            draw_downsample,
            draw_upsample,
            uniform_down_tex,
            uniform_down_size,
            uniform_up_tex,
            uniform_up_radius,
        };
        this.resize(width, height)?;
        Ok(this)
    }

    pub fn process(&self, texture: &Texture<[f32; 3]>, radius: f32) -> Result<&Texture<[f32; 3]>> {
        Framebuffer::disable_depth_test();
        Framebuffer::disable_blending();
        self.render_downsample(texture)?;
        self.render_upsample(radius)?;
        Ok(self.mip_chain.first().unwrap())
    }

    pub fn resize(&mut self, mut width: NonZeroU32, mut height: NonZeroU32) -> Result<()> {
        let depth = NonZeroU32::new(1).unwrap();
        self.mip_chain.iter_mut().try_for_each(|mip| {
            width = NonZeroU32::new(width.get() / 2).unwrap();
            height = NonZeroU32::new(height.get() / 2).unwrap();
            mip.clear_resize(width, height, depth)?;
            Ok::<_, eyre::Report>(())
        })?;
        Ok(())
    }

    fn render_downsample(&self, texture: &Texture<[f32; 3]>) -> Result<()> {
        let (w, h) = texture.mipmap_size(0)?;
        let size = UVec2::new(w.get(), h.get()).as_vec2();
        self.draw_downsample
            .set_uniform(self.uniform_down_size, size)?;
        self.draw_downsample
            .set_uniform(self.uniform_down_tex, texture.as_uniform(0)?)?;

        let mut first_mip = true;
        for mip in &self.mip_chain {
            let size = mip.size_vec().truncate();
            Framebuffer::viewport(0, 0, size.x as _, size.y as _);
            self.fbo.attach_color(0, mip)?;
            self.draw_downsample.set_uniform(self.draw_downsample.uniform("first_mip"), first_mip)?;
            self.draw_downsample.draw(&self.fbo)?;

            self.draw_downsample
                .set_uniform(self.uniform_down_size, size.as_vec2())?;
            self.draw_downsample
                .set_uniform(self.uniform_down_tex, mip.as_uniform(0)?)?;
            first_mip = false;
        }
        Ok(())
    }

    fn render_upsample(&self, radius: f32) -> Result<()> {
        self.draw_upsample
            .set_uniform(self.uniform_up_radius, radius)?;
        Framebuffer::enable_blending(Blend::One, Blend::One);
        Framebuffer::blend_equation(BlendFunction::Add);

        // for window in self.mip_chain.windows(2).rev() {
        for i in (1..self.mip_chain.len()).rev() {
            let mip = &self.mip_chain[i];
            let next_mip = &self.mip_chain[i - 1];
            // let mip = &window[1];
            // let next_mip = &window[0];
            let size = next_mip.size_vec().truncate();

            self.draw_upsample
                .set_uniform(self.uniform_up_tex, mip.as_uniform(0)?)?;
            Framebuffer::viewport(0, 0, size.x as _, size.y as _);
            self.fbo.attach_color(0, next_mip)?;
            self.draw_upsample.draw(&self.fbo)?;
        }
        Ok(())
    }
}
