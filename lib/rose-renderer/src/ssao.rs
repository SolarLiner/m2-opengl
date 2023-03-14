use std::num::NonZeroU32;

use eyre::Result;
use glam::{vec3, Vec3};
use rand::{thread_rng, Rng};
use rose_core::{
    camera::ViewUniformBuffer,
    screen_draw::ScreenDraw, utils::reload_watcher::ReloadWatcher,
};

use violette::{
    framebuffer::Framebuffer,
    program::{UniformBlockIndex, UniformLocation},
    texture::{Dimension, SampleMode, Texture},
};

#[derive(Debug)]
pub struct Ssao {
    fbo: Framebuffer,
    noise_texture: Texture<[f32; 3]>,
    target: Texture<f32>,
    draw: ScreenDraw,
    u_view: UniformBlockIndex,
    u_position: UniformLocation,
    u_albedo: UniformLocation,
    u_normal: UniformLocation,
    u_noise: UniformLocation,
    u_samples: UniformLocation,
}

impl Ssao {
    pub fn new(
        width: NonZeroU32,
        height: NonZeroU32,
        reload_watcher: &ReloadWatcher,
    ) -> Result<Self> {
        let noise = ssao_noise()
            .into_iter()
            .flat_map(|v| v.to_array())
            .collect::<Vec<_>>();
        let noise_texture = Texture::from_2d_pixels(NonZeroU32::new(4).unwrap(), &noise)?;
        noise_texture.filter_min(SampleMode::Nearest)?;
        noise_texture.filter_mag(SampleMode::Nearest)?;

        let target = Texture::new(width, height, NonZeroU32::new(1).unwrap(), Dimension::D2);
        target.reserve_memory()?;

        let fbo = Framebuffer::new();
        fbo.attach_color(0, target.mipmap(0).unwrap())?;

        let draw = ScreenDraw::load(
            reload_watcher.base_path().join("screen/ssao.glsl"),
            reload_watcher,
        )?;
        let program = draw.program();
        let u_view = program.uniform_block("View");
        let u_position = program.uniform("frame_position");
        let u_albedo = program.uniform("frame_albedo");
        let u_normal = program.uniform("frame_normal");
        let u_noise = program.uniform("noise");
        let u_samples = program.uniform("samples");
        program.set_uniform(u_samples, ssao_kernel().as_ref())?;
        drop(program);

        Ok(Self {
            fbo,
            noise_texture,
            target,
            draw,
            u_view,
            u_position,
            u_albedo,
            u_normal,
            u_noise,
            u_samples,
        })
    }

    #[inline(always)]
    pub fn render_target(&self) -> &Texture<f32> {
        &self.target
    }

    pub fn process(
        &self,
        view: &ViewUniformBuffer,
        position: &Texture<[f32; 3]>,
        albedo: &Texture<[f32; 3]>,
        normal: &Texture<[f32; 4]>,
    ) -> Result<&Texture<f32>> {
        {
            let program = self.draw.program();
            program.bind_block(&view.slice(0..=0), self.u_view, 0)?;
            program.set_uniform(self.u_position, position.as_uniform(0)?)?;
            program.set_uniform(self.u_albedo, albedo.as_uniform(1)?)?;
            program.set_uniform(self.u_normal, normal.as_uniform(2)?)?;
            program.set_uniform(self.u_noise, self.noise_texture.as_uniform(3)?)?;
        }
        let (width, height, _) = self.target.size();
        Framebuffer::viewport(0, 0, width.get() as _, height.get() as _);
        self.draw.draw(&self.fbo)?;
        Ok(&self.target)
    }

    pub fn set_kernel_size(&self, size: i32) -> Result<()> {
        let program = self.draw.program();
        let u_kernel_size = program.uniform("kernel_size");
        program.set_uniform(u_kernel_size, size)?;
        Ok(())
    }

    pub fn set_radius(&self, radius: f32) -> Result<()> {
        let program = self.draw.program();
        let u_radius = program.uniform("radius");
        program.set_uniform(u_radius, radius)?;
        Ok(())
    }

    pub fn resize(&mut self, width: NonZeroU32, height: NonZeroU32) -> Result<()> {
        self.target
            .clear_resize(width, height, NonZeroU32::new(1).unwrap())?;
        Ok(())
    }
}

fn ssao_kernel() -> [Vec3; 64] {
    let mut rng = thread_rng();
    std::array::from_fn(|i| {
        let scale = i as f32 / 64.;
        let scale = lerp(0.1, 1., scale * scale);
        vec3(
            rng.gen_range(-1f32..1.),
            rng.gen_range(-1f32..1.),
            rng.gen_range(0f32..1.),
        )
        .normalize()
            * rng.gen_range(0f32..1.)
    })
}

fn ssao_noise() -> [Vec3; 16] {
    let mut rng = thread_rng();
    std::array::from_fn(|_| vec3(rng.gen_range(-1f32..1.), rng.gen_range(-1f32..1.), 0.))
}

#[inline(always)]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    return a + t * (b - a);
}
