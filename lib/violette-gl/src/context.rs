use std::error::Error;
use cgmath::Vector2;
use glutin::context::PossiblyCurrentContext;
use glutin::prelude::*;
use glutin::surface::{Surface, WindowSurface};
use winit::window::Window;

pub struct WindowDesc {
    pub name: String,
    pub logical_size: Vector2<f32>,
}

impl Default for WindowDesc {
    fn default() -> Self {
        Self {
            name: "violette".to_string(),
            logical_size: Vector2::new(800., 600.),
        }
    }
}

pub struct OpenGLContext {
    window: Window,
    gl_context: PossiblyCurrentContext,
    gl_surface: Surface<WindowSurface>,
}

impl OpenGLContext {
    pub fn new(desc: WindowDesc) -> Result<Self, Box<dyn Error>> {
        todo!()
    }

    pub fn swap_buffers(&self) {
        self.gl_surface.swap_buffers(&self.gl_context).unwrap();
    }
}
