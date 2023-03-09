use std::f32::consts::PI;

use eyre::Result;
use glam::{EulerRot, Quat, vec2, Vec2};

use rose_core::camera::{Camera, ViewUniform, ViewUniformBuffer};
use rose_core::screen_draw::ScreenDraw;
use rose_core::utils::thread_guard::ThreadGuard;
use rose_platform::{Application, PhysicalSize, RenderContext, TickContext};
use rose_platform::events::{MouseScrollDelta, WindowEvent};
use violette::framebuffer::Framebuffer;
use violette::program::UniformBlockIndex;
use violette::texture::Texture;

struct TestMvp {
    win_size: Vec2,
    draw: ThreadGuard<ScreenDraw>,
    envmap: ThreadGuard<Texture<[f32; 3]>>,
    rotation: Vec2,
    camera: Camera,
    u_view: UniformBlockIndex,
    view_uniform: ThreadGuard<ViewUniformBuffer>,
}

impl TestMvp {
    fn get_rotation(&self) -> Quat {
        Quat::from_euler(EulerRot::YXZ, self.rotation.x, -self.rotation.y, 0.)
    }
}

impl Application for TestMvp {
    fn new(size: PhysicalSize<f32>, _scale_factor: f64) -> Result<Self> {
        let draw = ScreenDraw::load("assets/shaders/env/equirectangular.bg.glsl")?;
        let texture = Texture::load_rgb32f("assets/textures/table_mountain_2_puresky_4k.exr")?;
        let u_view = draw.uniform_block("View");
        draw.set_uniform(draw.uniform("env_map"), texture.as_uniform(0)?)?;
        let mut camera = Camera::default();
        camera.projection.update(Vec2::from_array(size.into()));
        camera.transform.rotation = Quat::from_euler(EulerRot::YXZ, 0., 0.5, 0.);
        let view_uniform = ViewUniform::from(camera.clone()).create_buffer()?;
        draw.bind_block(&view_uniform.slice(0..=0), u_view, 0)?;
        Ok(Self {
            win_size: Vec2::from_array(size.into()),
            draw: ThreadGuard::new(draw),
            envmap: ThreadGuard::new(texture),
            camera,
            view_uniform: ThreadGuard::new(view_uniform),
            rotation: vec2(0., 0.5),
            u_view,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>, _scale_factor: f64) -> Result<()> {
        self.win_size = Vec2::from_array(size.into());
        self.camera.projection.update(self.win_size);
        let size = size.cast();
        Framebuffer::viewport(0, 0, size.width, size.height);
        Ok(())
    }

    fn interact(&mut self, _event: WindowEvent) -> Result<()> {
        match _event {
            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    MouseScrollDelta::LineDelta(_, y) => self.camera.projection.fovy += 1e-2 * y,
                    MouseScrollDelta::PixelDelta(s) => {
                        self.camera.projection.fovy += s.y as f32 / self.win_size.y
                    }
                }
                self.camera.projection.fovy = self.camera.projection.fovy.clamp(1e-3, PI - 1e-3);
                tracing::info!("FOV y: {}", self.camera.projection.fovy);
                let vd = ViewUniform::from(self.camera.clone());
                tracing::info!("inv_proj {}", vd.inv_proj);
            }
            _ => {}
        }
        Ok(())
    }

    fn tick(&mut self, ctx: TickContext) -> Result<()> {
        let t = ctx.elapsed.as_secs_f32() * 0.4;
        self.rotation.x = t;
        self.rotation.y = t.sin();
        Ok(())
    }

    fn render(&mut self, _ctx: RenderContext) -> Result<()> {
        self.camera.transform.rotation = self.get_rotation();
        ViewUniform::from(self.camera.clone()).update_uniform_buffer(&mut self.view_uniform)?;
        let backbuffer = Framebuffer::backbuffer();
        // self.draw
        //     .bind_block(self.u_view, &self.view_uniform.slice(0..=0))?;
        self.draw.draw(&backbuffer)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    rose_platform::run::<TestMvp>("Test MVP")
}
