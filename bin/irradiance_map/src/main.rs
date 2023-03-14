use std::{
    num::NonZeroU32,
    path::{Path, PathBuf},
};

use rose::{core::utils::reload_watcher::ReloadWatcher, prelude::*};
use violette::{
    framebuffer::{ClearBuffer, Framebuffer},
    program::UniformLocation,
    texture::{Dimension, SampleMode, Texture},
};

struct IrradianceMapImpl {
    fbo: Framebuffer,
    environment_map: Option<Texture<[f32; 3]>>,
    irradiance_texture: Option<Texture<[f32; 3]>>,
    make_irradiance: ScreenDraw,
    display_texture: ScreenDraw,
    u_irradiance_env_map: UniformLocation,
    u_display_texture: UniformLocation,
    reload_watcher: ReloadWatcher,
}

impl IrradianceMapImpl {
    fn load_file(&mut self, path: impl AsRef<Path>) -> eyre::Result<()> {
        let environment_map = Texture::load_rgb32f(path)?;
        let texture = Texture::new(
            NonZeroU32::new(256).unwrap(),
            NonZeroU32::new(128).unwrap(),
            NonZeroU32::new(1).unwrap(),
            Dimension::D2,
        );
        texture.reserve_memory()?;
        texture.filter_min(SampleMode::Linear)?;
        texture.filter_mag(SampleMode::Linear)?;

        self.fbo.attach_color(0, texture.mipmap(0).unwrap())?;
        self.fbo.assert_complete()?;

        self.make_irradiance
            .program()
            .set_uniform(self.u_irradiance_env_map, environment_map.as_uniform(0)?)?;
        Framebuffer::viewport(0, 0, 256, 128);
        self.make_irradiance.draw(&self.fbo)?;

        self.environment_map.replace(environment_map);
        self.irradiance_texture.replace(texture);
        Ok(())
    }
}

struct IrradianceMap(ThreadGuard<IrradianceMapImpl>);

impl Application for IrradianceMap {
    fn window_features(wb: WindowBuilder) -> WindowBuilder {
        wb.with_inner_size(LogicalSize::new(1024, 512))
            .with_resizable(false)
    }

    fn new(size: PhysicalSize<f32>, scale_factor: f64) -> eyre::Result<Self> {
        let reload_watcher = ReloadWatcher::new("res/shaders");
        let make_irradiance = ScreenDraw::load("screen/env/irradiance.glsl", &reload_watcher)?;
        let display_texture = ScreenDraw::load("blit.glsl", &reload_watcher)?;
        let fbo = Framebuffer::new();
        let u_display_texture = display_texture.program().uniform("in_texture");
        let u_irradiance_env_map = make_irradiance.program().uniform("env_map");
        let mut this = Self(ThreadGuard::new(IrradianceMapImpl {
            irradiance_texture: None,
            fbo,
            display_texture,
            environment_map: None,
            make_irradiance,
            u_irradiance_env_map,
            u_display_texture,
            reload_watcher,
        }));
        if let Some(path) = std::env::args()
            .nth(1)
            .map(|s| s.parse::<PathBuf>().unwrap())
        {
            this.0.load_file(path)?;
        }
        Ok(this)
    }

    fn interact(&mut self, _event: WindowEvent) -> eyre::Result<()> {
        match _event {
            WindowEvent::DroppedFile(path) => {
                self.0.load_file(path)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn render(&mut self, ctx: RenderContext) -> eyre::Result<()> {
        let size = ctx.window.inner_size().cast();
        Framebuffer::viewport(0, 0, size.width, size.height);
        if let Some(irradiance_texture) = &self.0.irradiance_texture {
            self.0
                .display_texture
                .program()
                .set_uniform(self.0.u_display_texture, irradiance_texture.as_uniform(0)?)?;
            self.0.display_texture.draw(&Framebuffer::backbuffer())?;
        } else {
            Framebuffer::clear_color([0., 0., 0., 1.]);
            Framebuffer::backbuffer().do_clear(ClearBuffer::COLOR);
        }
        Ok(())
    }

    #[cfg(never)]
    fn render(&mut self, ctx: RenderContext) -> eyre::Result<()> {
        if let Some(envmap) = &self.0.environment_map {
            self.0
                .make_irradiance
                .program()
                .set_uniform(self.0.u_irradiance_env_map, envmap.as_uniform(0)?)?;
            self.0.make_irradiance.draw(&Framebuffer::backbuffer())?;
        } else {
            Framebuffer::clear_color([0., 0., 0., 1.]);
            Framebuffer::backbuffer().do_clear(ClearBuffer::COLOR);
        }
        Ok(())
    }
}

fn main() -> eyre::Result<()> {
    run::<IrradianceMap>("Irradiance map calc")
}
