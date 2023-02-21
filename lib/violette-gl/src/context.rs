use std::error::Error;
use std::sync::Arc;

use cgmath::Vector2;
use crevice::std140::AsStd140;
use glutin::context::PossiblyCurrentContext;
use glutin::prelude::*;
use glutin::surface::{Surface, WindowSurface};
use winit::window::Window;

use violette as api;
use violette::{BufferKind, ClearBuffers, Color, Rect};
use crate::api::{OpenGLApi, OpenGLError};

use crate::arrays::VertexArray;
use crate::buffer::Buffer;
use crate::framebuffer::Framebuffer;
use crate::thread_guard::ThreadGuard;

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

#[derive(Debug)]
pub struct OpenGLContextImpl {
    gl_context: PossiblyCurrentContext,
    gl_surface: Surface<WindowSurface>,
}

#[derive(Debug)]
pub struct OpenGLContext(ThreadGuard<OpenGLContextImpl>);

impl OpenGLContext {
    pub(crate) fn new(context: PossiblyCurrentContext, surface: Surface<WindowSurface>) -> Self {
        let inner = OpenGLContextImpl {
            gl_context: context,
            gl_surface: surface,
        };
        Self(ThreadGuard::new(inner))
    }
}

impl api::GraphicsContext for OpenGLContext {
    type Err = OpenGLError;
    type Api = OpenGLApi;
    type Buffer<T: AsStd140> = ThreadGuard<Buffer<T>>;
    type Framebuffer = ThreadGuard<Framebuffer>;
    type ShaderModule = ThreadGuard<Program>;
    type VertexArray = ThreadGuard<VertexArray>;

    fn swap_buffers(&self) {
        self.gl_surface.swap_buffers(&self.gl_context).unwrap();
    }

    fn create_buffer<T: AsStd140>(
        &self,
        kind: BufferKind,
    ) -> Result<Arc<Self::Buffer<T>>, Self::Err> {
        OK(Arc::new(ThreadGuard::new(Buffer::new(kind))))
    }

    fn create_framebuffer(&self) -> Result<Arc<Self::Framebuffer>, Self::Err> {
        Ok(Arc::new(ThreadGuard::new(Framebuffer::new())))
    }

    fn create_shader_module(&self) -> Result<Arc<Self::ShaderModule>, Self::Err> {
        Ok(Arc::new(ThreadGuard::new(Program::new())))
    }

    fn create_vertex_array(&self) -> Result<Arc<Self::VertexArray>, Self::Err> {
        Ok(Arc::new(ThreadGuard::new(VertexArray::new())))
    }

    fn viewport(&self, rect: Rect<f32>) {
        unsafe {
            let [x, y, w, h] = rect.cast().into();
            gl::Viewport(x, y, w, h);
        }
    }

    fn set_depth_test(&self, enabled: bool) {
        if enabled {
            unsafe { gl::Enable(gl::DEPTH_TEST) };
        } else {
            unsafe { gl::Disable(gl::DEPTH_TEST) };
        }
    }

    fn set_scissor_test(&self, enabled: bool) {
        if enabled {
            unsafe { gl::Enable(gl::SCISSOR_TEST) };
        } else {
            unsafe { gl::Disable(gl::SCISSOR_TEST) };
        }
    }

    fn set_clear_color(&self, color: Color) {
        let [r, g, b, a] = color.into();
        unsafe {
            gl::ClearColor(r, g, b, a);
        }
    }

    fn set_clear_depth(&self, depth: f64) {
        unsafe {
            gl::ClearDepth(depth);
        }
    }

    fn set_clear_stencil(&self, stencil: i32) {
        unsafe { gl::ClearStencil(stencil) }
    }

    fn set_line_width(&self, width: f32) {
        unsafe {
            gl::LineWidth(width);
        }
    }

    fn clear(&self, mode: ClearBuffers) {
        let mut bits = 0;
        if mode.contains(ClearBuffers::COLOR) {
            bits |= gl::COLOR_BUFFER_BIT;
        }
        if mode.contains(ClearBuffers::DEPTH) {
            bits |= gl::DEPTH_BUFFER_BIT;
        }
        if mode.contains(ClearBuffers::STENCIL) {
            bits |= gl::STENCIL_BUFFER_BIT;
        }
        unsafe {
            gl::Clear(bits);
        }
    }
}
