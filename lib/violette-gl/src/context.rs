use std::ffi::CString;
use std::sync::Weak;
use std::{
    fmt::{self, Formatter},
    sync::Arc,
};

use bytemuck::Pod;
use glutin::{
    context::{
        PossiblyCurrentContext,
        ContextApi,
        ContextAttributesBuilder,
        GlProfile,
        Robustness,
        Version
    },
    display::GetGlDisplay,
    prelude::*,
    surface::{
        SurfaceAttributesBuilder,
        Surface,
        WindowSurface
    }
};
use violette_api::math::glm::TVec2 as Vector2;
use raw_window_handle::HasRawWindowHandle;
use thiserror::Error;

use violette_api::{
    buffer::BufferKind,
    context::{ClearBuffers, GraphicsContext},
    math::{Color, Rect},
    window::Window as ApiWindow,
};

use crate::{
    api::{OpenGLApi, OpenGLError},
    arrays::VertexArray,
    buffer::Buffer,
    framebuffer::Framebuffer,
    program::Program,
    thread_guard::ThreadGuard,
    window::OpenGLWindow,
    Gl,
};

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("Windowing error: {0}")]
    Glutin(#[from] glutin::error::Error),
    #[error("OpenGL error: {0}")]
    OpenGL(#[from] OpenGLError),
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

pub struct OpenGLContextImpl {
    gl: Gl,
    gl_context: Arc<ThreadGuard<PossiblyCurrentContext>>,
    gl_surface: Surface<WindowSurface>,
}

impl fmt::Debug for OpenGLContextImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("OpenGLContextImpl").finish()
    }
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
        tracing::debug!("Load OpenGL symbols");
        let gl = crate::load_with(|sym| {
            window
                .config()
                .display()
                .get_proc_address(CString::new(sym).unwrap().as_c_str())
        });
        tracing::debug!("Set OpenGL debug message callbacks");
        crate::debug::set_message_callback(&gl, |data| {
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
        Ok(Self {
            gl,
            gl_surface: surface,
            gl_context: context,
        })
    }

    fn make_current(&self) -> Result<(), ContextError> {
        self.gl_context.make_current(&self.gl_surface)?;
        Ok(())
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
        let inner = OpenGLContextImpl::create(&window)?;
        let backbuffer = Arc::new(Framebuffer::backbuffer(&inner.gl));
        Ok(Self {
            ctx_impl: ThreadGuard::new(inner),
            backbuffer,
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

    pub(crate) fn make_current(&self) -> Result<(), ContextError> {
        self.ctx_impl.make_current()
    }
}

impl GraphicsContext for OpenGLContext {
    type Window = OpenGLWindow;
    type Err = OpenGLError;
    type Buffer<T: 'static + Send + Sync + Pod> = Buffer<T>;
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

    fn create_buffer<T: 'static + Send + Sync + Pod>(
        &self,
        kind: BufferKind,
    ) -> Result<Arc<Self::Buffer<T>>, Self::Err> {
        Ok(Arc::new(Buffer::new(self.gl(), kind)))
    }

    fn create_vertex_array(&self) -> Result<Arc<Self::VertexArray>, Self::Err> {
        Ok(Arc::new(VertexArray::new(self.gl())))
    }

    fn create_shader_module(&self) -> Result<Arc<Self::ShaderModule>, Self::Err> {
        Ok(Arc::new(Program::new(self.gl())?))
    }

    fn create_framebuffer(&self) -> Result<Arc<Self::Framebuffer>, Self::Err> {
        Ok(Arc::new(Framebuffer::new(self.gl())))
    }

    fn swap_buffers(&self) {
        self.ctx_impl
            .gl_surface
            .swap_buffers(&self.ctx_impl.gl_context)
            .unwrap();
    }
}

impl OpenGLContext {
    fn gl(&self) -> &Gl {
        &self.ctx_impl.gl
    }
}
