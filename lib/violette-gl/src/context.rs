use std::error::Error;
use std::fmt;
use std::fmt::Formatter;
use std::ops::Deref;
use std::sync::Arc;

use cgmath::Vector2;
use crevice::std140::AsStd140;
use glutin::{
    context::PossiblyCurrentContext,
    prelude::*,
    surface::{Surface, WindowSurface},
};
use winit::window::Window;

use violette_api::context::GraphicsContext;
use violette_api::{
    buffer::BufferKind,
    context::ClearBuffers,
    math::{Color, Rect},
};

use crate::{api::{OpenGLApi, OpenGLError}, Gl};
use crate::arrays::VertexArray;
use crate::buffer::Buffer;
use crate::framebuffer::{Framebuffer, FramebufferImpl};
use crate::program::Program;
use crate::thread_guard::ThreadGuard;

pub struct WindowDesc {
    pub name: String,
    pub logical_size: Vector2<f32>,
}

impl Default for WindowDesc {
    fn default() -> Self {
        Self {
            name: "violette-old".to_string(),
            logical_size: Vector2::new(800., 600.),
        }
    }
}

pub struct OpenGLContextImpl {
    gl: Gl,
    gl_context: Arc<ThreadGuard<PossiblyCurrentContext>>,
    gl_surface: Surface<WindowSurface>,
}

impl fmt::Debug for OpenGLContextImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenGLContextImpl")
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct OpenGLContext {
    ctx_impl: ThreadGuard<OpenGLContextImpl>,
    backbuffer: Arc<Framebuffer>,
}

impl OpenGLContext {
    pub(crate) fn new(
        gl: Gl,
        context: Arc<ThreadGuard<PossiblyCurrentContext>>,
        surface: Surface<WindowSurface>,
    ) -> Self {
        let inner = OpenGLContextImpl {
            gl,
            gl_context: context,
            gl_surface: surface,
        };
        Self {
            ctx_impl: ThreadGuard::new(inner),
            backbuffer: Arc::new(Framebuffer(ThreadGuard::new(FramebufferImpl::backbuffer()))),
        }
    }
}

impl GraphicsContext for OpenGLContext {
    type Api = OpenGLApi;
    type Err = OpenGLError;
    type Buffer<T:'static + Send + Sync + AsStd140> = Buffer<T>;
    type Framebuffer = Framebuffer;
    type VertexArray = VertexArray;
    type ShaderModule = Program;

    fn backbuffer(&self) -> Arc<Self::Framebuffer> {
        self.backbuffer.clone()
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
            self.gl().Clear(bits);
        }
    }

    fn set_line_width(&self, width: f32) {
        unsafe {
            self.gl().LineWidth(width);
        }
    }

    fn set_clear_stencil(&self, stencil: i32) {
        unsafe { self.gl().ClearStencil(stencil) }
    }

    fn set_clear_depth(&self, depth: f64) {
        unsafe {
            self.gl().ClearDepth(depth);
        }
    }

    fn set_clear_color(&self, color: Color) {
        let [r, g, b, a] = color.into_array();
        unsafe {
            self.gl().ClearColor(r, g, b, a);
        }
    }

    fn set_scissor_test(&self, enabled: bool) {
        if enabled {
            unsafe { self.gl().Enable(gl::SCISSOR_TEST) };
        } else {
            unsafe { self.gl().Disable(gl::SCISSOR_TEST) };
        }
    }

    fn set_depth_test(&self, enabled: bool) {
        if enabled {
            unsafe { self.gl().Enable(gl::DEPTH_TEST) };
        } else {
            unsafe { self.gl().Disable(gl::DEPTH_TEST) };
        }
    }

    fn viewport(&self, rect: Rect<f32>) {
        unsafe {
            let [x, y, w, h] = rect.cast().into_array();
            self.gl().Viewport(x, y, w, h);
        }
    }

    fn create_buffer<T: 'static + AsStd140>(
        &self,
        kind: BufferKind,
    ) -> Result<Arc<Self::Buffer<T>>, Self::Err> {
        Ok(Arc::new(Buffer::new(&self.gl(), kind)))
    }

    fn create_vertex_array(&self) -> Result<Arc<Self::VertexArray>, Self::Err> {
        Ok(Arc::new(VertexArray::new(&self.gl())))
    }

    fn create_shader_module(&self) -> Result<Arc<Self::ShaderModule>, Self::Err> {
        Ok(Arc::new(Program::new()?))
    }

    fn create_framebuffer(&self) -> Result<Arc<Self::Framebuffer>, Self::Err> {
        Ok(Arc::new(Framebuffer::new(self.gl())))
    }

    fn swap_buffers(&self) {
        self.ctx_impl.gl_surface.swap_buffers(&self.ctx_impl.gl_context).unwrap();
    }
}

impl OpenGLContext {
    fn gl(&self) -> &Gl {
        &self.ctx_impl.gl
    }
}
