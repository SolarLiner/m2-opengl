use std::ffi::CString;
use std::sync::{Arc, Weak};

use cgmath::Vector2;
use crevice::std140::AsStd140;
use glutin::context::{ContextApi, ContextAttributesBuilder, GlProfile, Robustness, Version};
use glutin::display::GetGlDisplay;
use glutin::surface::SurfaceAttributesBuilder;
use glutin::{
    context::PossiblyCurrentContext,
    prelude::*,
    surface::{Surface, WindowSurface},
};
use once_cell::sync::OnceCell;
use raw_window_handle::HasRawWindowHandle;
use thiserror::Error;

use violette_api::window::Window;
use violette_api::{
    buffer::BufferKind,
    context::ClearBuffers,
    context::GraphicsContext,
    math::{Color, Rect},
};

use crate::{
    api::OpenGLError,
    arrays::VertexArray,
    buffer::Buffer,
    framebuffer::{Framebuffer, FramebufferImpl},
    program::Program,
    thread_guard::ThreadGuard,
    window::OpenGLWindow,
};

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("OpenGL context error: {0}")]
    Glutin(#[from] glutin::error::Error),
    #[error("OpenGL error: {0}")]
    OpenGl(#[from] OpenGLError),
}

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

#[derive(Debug)]
pub struct OpenGLContextImpl {
    gl_context: Arc<ThreadGuard<PossiblyCurrentContext>>,
    gl_surface: Surface<WindowSurface>,
}

impl OpenGLContextImpl {
    fn create(window: &OpenGLWindow) -> Result<Self, ContextError> {
        let size = window.physical_size();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window.raw_window_handle(),
            size.x.try_into().unwrap(),
            size.y.try_into().unwrap(),
        );
        let config = window.config();
        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .with_profile(GlProfile::Core)
            .with_robustness(Robustness::RobustLoseContextOnReset)
            .with_debug(cfg!(debug_assertions))
            .build(Some(window.raw_window_handle()));
        let context = unsafe { config.display().create_context(config, &context_attributes) }?;
        let surface = unsafe { config.display().create_window_surface(&config, &attrs) }?;
        let context = Arc::new(ThreadGuard::new(context.make_current(&surface)?));
        Ok(Self {
            gl_surface: surface,
            gl_context: context,
        })
    }
}

#[derive(Debug)]
pub struct OpenGLContext {
    ctx_impl: ThreadGuard<OpenGLContextImpl>,
    window: Weak<OpenGLWindow>,
    backbuffer: Arc<Framebuffer>,
}

impl OpenGLContext {
    pub(crate) fn new(window: Arc<OpenGLWindow>) -> Result<Self, ContextError> {
        static LOADED: OnceCell<()> = OnceCell::new();
        let inner = OpenGLContextImpl::create(&window)?;
        LOADED.get_or_init(|| {
            tracing::debug!("Load OpenGL symbols");
            gl::load_with(|sym| {
                window
                    .config()
                    .display()
                    .get_proc_address(CString::new(sym).unwrap().as_c_str())
            });
            tracing::debug!("Set OpenGL debug message callbacks");
            crate::debug::set_message_callback(|data| {
        use crate::debug::CallbackSeverity::*;
        match data.severity {
            Notification => {
                tracing::debug!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type)
            }
            Low => {
                tracing::info!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type)
            }
            Medium => {
                tracing::warn!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type)
            }
            High => {
                tracing::error!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type)
            }
        };
    });
        });
        Ok(Self {
            ctx_impl: ThreadGuard::new(inner),
            backbuffer: Arc::new(Framebuffer(ThreadGuard::new(FramebufferImpl::backbuffer()))),
            window: Arc::downgrade(&window),
        })
    }

    pub(crate) fn resize(&self, size: Vector2<u32>) {
        tracing::debug!(message="Context resize", size=?size);
        self.ctx_impl.gl_surface.resize(
            &self.ctx_impl.gl_context,
            size.x.try_into().unwrap(),
            size.y.try_into().unwrap(),
        );
    }
}

impl GraphicsContext for OpenGLContext {
    type Window = OpenGLWindow;
    type Err = ContextError;
    type Buffer<T: 'static + AsStd140> = Buffer<T>;
    type Framebuffer = Framebuffer;
    type VertexArray = VertexArray;
    type ShaderModule = Program;

    fn window(&self) -> Weak<Self::Window> {
        self.window.clone()
    }

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
            gl::Clear(bits);
        }
    }

    fn set_line_width(&self, width: f32) {
        unsafe {
            gl::LineWidth(width);
        }
    }

    fn set_clear_stencil(&self, stencil: i32) {
        unsafe { gl::ClearStencil(stencil) }
    }

    fn set_clear_depth(&self, depth: f64) {
        unsafe {
            gl::ClearDepth(depth);
        }
    }

    fn set_clear_color(&self, color: Color) {
        let [r, g, b, a] = color.into_array();
        unsafe {
            gl::ClearColor(r, g, b, a);
        }
    }

    fn set_scissor_test(&self, enabled: bool) {
        if enabled {
            unsafe { gl::Enable(gl::SCISSOR_TEST) };
        } else {
            unsafe { gl::Disable(gl::SCISSOR_TEST) };
        }
    }

    fn set_depth_test(&self, enabled: bool) {
        if enabled {
            unsafe { gl::Enable(gl::DEPTH_TEST) };
        } else {
            unsafe { gl::Disable(gl::DEPTH_TEST) };
        }
    }

    fn viewport(&self, rect: Rect<f32>) {
        unsafe {
            let [x, y, w, h] = rect.cast().into_array();
            gl::Viewport(x, y, w, h);
        }
    }

    fn make_current(&self) {
        self.ctx_impl
            .gl_context
            .make_current(&self.ctx_impl.gl_surface)
            .unwrap();
    }

    fn create_buffer<T: 'static + AsStd140>(
        &self,
        kind: BufferKind,
    ) -> Result<Arc<Self::Buffer<T>>, Self::Err> {
        Ok(Arc::new(Buffer::new(kind)))
    }

    fn create_vertex_array(&self) -> Result<Arc<Self::VertexArray>, Self::Err> {
        Ok(Arc::new(VertexArray::new()))
    }

    fn create_shader_module(&self) -> Result<Arc<Self::ShaderModule>, Self::Err> {
        Ok(Arc::new(Program::new()?))
    }

    fn create_framebuffer(&self) -> Result<Arc<Self::Framebuffer>, Self::Err> {
        Ok(Arc::new(Framebuffer::new()))
    }

    fn swap_buffers(&self) {
        self.ctx_impl
            .gl_surface
            .swap_buffers(&self.ctx_impl.gl_context)
            .unwrap();
    }
}
