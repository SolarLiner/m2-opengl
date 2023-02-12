use std::time::{Duration, Instant};

use eyre::Result;
use winit::dpi::PhysicalSize;

use m2_opengl::Application;
use m2_opengl::screen_draw::ScreenDraw;
use violette_low::framebuffer::Framebuffer;

const DRAW_SOURCE: &str = r#"
#version 330

uniform vec3 u_color;
out vec3 color;

void main() {
   color = u_color;
}"#;

struct TestApp {
    start: Instant,
    drawable: ScreenDraw,
}

impl Application for TestApp {
    fn new(_: PhysicalSize<f32>) -> Result<Self> {
        let drawable = ScreenDraw::new(DRAW_SOURCE)?;
        drawable.set_uniform(drawable.uniform("u_color").unwrap(), [1., 0., 1.])?;
        Ok(Self { start: Instant::now(), drawable })
    }

    fn tick(&mut self, _: Duration) {
        let (s, c) = self.start.elapsed().as_secs_f32().sin_cos();
        self.drawable.set_uniform(self.drawable.uniform("u_color").unwrap(), [c, s, 1.]).unwrap();
    }

    fn render(&mut self) {
        let frame = &*Framebuffer::backbuffer();
        self.drawable.draw(frame).unwrap();
    }
}

fn main() -> Result<()> {
    m2_opengl::run::<TestApp>("Test render")
}