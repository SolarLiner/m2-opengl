use std::backtrace::{Backtrace, BacktraceStatus};
use std::cell::{Cell, RefCell};
use std::fmt::Formatter;
use std::time::Duration;
use std::{
    error::Error,
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use std::ffi::CString;

use cgmath::num_traits;
use crevice::std140::AsStd140;
use glutin::config::GetGlConfig;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use glutin::{
    config::{Config, ConfigTemplateBuilder},
    context::{ContextApi, ContextAttributesBuilder, NotCurrentContext, Version},
    display::{Display, DisplayFeatures, GetGlDisplay},
    prelude::*,
};
use glutin_winit::DisplayBuilder;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use raw_window_handle::HasRawWindowHandle;
use thiserror::Error;
use winit::event::VirtualKeyCode::Back;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::{dpi::LogicalSize, event_loop::EventLoop, window::Fullscreen, window::WindowBuilder};

use violette_api::window::Window;
use violette_api::{api::Api, window::WindowDesc};
use violette_api::base::Resource;

use crate::buffer::Buffer;
use crate::thread_guard::ThreadGuard;
use crate::window::WindowError;
use crate::{context::OpenGLContext, Gl, window::OpenGLWindow};

#[derive(Debug, Copy, Clone, Error, FromPrimitive)]
#[repr(u32)]
pub enum GlErrorKind {
    #[error("Provided enum value is not valid")]
    InvalidEnum = gl::INVALID_ENUM,
    #[error("Provided value is not valid")]
    InvalidValue = gl::INVALID_VALUE,
    #[error("Invalid OpenGL operation")]
    InvalidOperation = gl::INVALID_OPERATION,
    #[error("Stack Overflow")]
    StackOverflow = gl::STACK_OVERFLOW,
    #[error("Stack Underflow")]
    StackUnderflow = gl::STACK_UNDERFLOW,
    #[error("Out of memory")]
    OutOfMemory = gl::OUT_OF_MEMORY,
    #[error("Invalid OpenGL operation on the framebuffer")]
    InvalidFramebufferOperation = gl::INVALID_FRAMEBUFFER_OPERATION,
    #[error("Context lost")]
    ContextLost = gl::CONTEXT_LOST,
    #[error("Unknown OpenGL error")]
    UnknownError,
}

impl GlErrorKind {
    pub fn current_error(gl: &Gl) -> Option<Self> {
        let error = unsafe { gl.GetError() };
        (error != gl::NO_ERROR)
            .then(|| GlErrorKind::from_u32(error).unwrap_or(GlErrorKind::UnknownError))
    }
}

#[derive(Debug)]
pub struct OpenGLError {
    pub kind: GlErrorKind,
    pub info: String,
    pub backtrace: Backtrace,
}

impl From<GlErrorKind> for OpenGLError {
    fn from(value: GlErrorKind) -> Self {
        Self {
            kind: value,
            info: "".to_string(),
            backtrace: Backtrace::capture(),
        }
    }
}

impl Error for OpenGLError {}

impl fmt::Display for OpenGLError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.info)
    }
}

impl OpenGLError {
    pub fn with_info_log(gl: &Gl, info: impl ToString) -> Option<Self> {
        GlErrorKind::current_error(&gl).map(|kind| Self {
            kind,
            info: info.to_string(),
            backtrace: Backtrace::capture(),
        })
    }

    pub fn guard(gl: &Gl) -> Result<(), Self> {
        if let Some(kind) = GlErrorKind::current_error(&gl) {
            Err(Self {
                kind,
                info: kind.to_string(),
                backtrace: Backtrace::capture(),
            })
        } else {
            Ok(())
        }
    }
}

pub struct OpenGLApi {
    event_loop: ThreadGuard<RefCell<Option<EventLoop<()>>>>,
}

impl OpenGLApi {
    pub fn new() -> Arc<Self> {
        let event_loop = EventLoop::new();
        Arc::new(Self {
            event_loop: ThreadGuard::new(RefCell::new(Some(event_loop))),
        })
    }
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("OpenGL error: {0}")]
    OpenGL(#[from] OpenGLError),
    #[error("Glutin context error: {0}")]
    Glutin(#[from] glutin::error::Error),
    #[error("Windowing error: {0}")]
    Window(#[from] WindowError),
    #[error("Platform error: {0}")]
    Platform(#[from] WinitError),
}

#[derive(Debug)]
pub struct WinitError {
    inner: ThreadGuard<Box<dyn Error>>,
}

impl Error for WinitError {}

impl fmt::Display for WinitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &*self.inner)
    }
}

impl WinitError {
    fn from_dyn_error(err: Box<dyn Error>) -> Self {
        Self {
            inner: ThreadGuard::new(err),
        }
    }
}

impl Api for OpenGLApi {
    type Err = ApiError;
    type Buffer<T: AsStd140> = Buffer<T>;
    type Window = OpenGLWindow;
    type GraphicsContext = OpenGLContext;

    fn create_graphics_context(
        self: Arc<Self>,
        window: Arc<Self::Window>,
    ) -> Result<Self::GraphicsContext, Self::Err> {
        let context = window.context();
        let gl = crate::load_with(|sym| unsafe {
            let sym = CString::new(sym).unwrap();
            context.display().get_proc_address(sym.as_c_str())
        });
        let size = window.physical_size();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window.raw_window_handle(),
            size.x.try_into().unwrap(),
            size.y.try_into().unwrap(),
        );
        let surface = unsafe {
            context
                .display()
                .create_window_surface(&context.config(), &attrs)
        }?;
        Ok(OpenGLContext::new(gl, context, surface))
    }

    fn create_window(self: Arc<Self>, desc: WindowDesc) -> Result<Arc<Self::Window>, Self::Err> {
        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(true);
        let display_builder = DisplayBuilder::new().with_window_builder(Some({
            let wb = if let Some(str) = desc.title.as_deref() {
                WindowBuilder::new().with_title(str)
            } else {
                WindowBuilder::new()
            };
            wb.with_inner_size(LogicalSize::new(desc.logical_size.x, desc.logical_size.y))
                .with_fullscreen(desc.fullscreen.then_some(Fullscreen::Borderless(None)))
        }));
        let event_loop = self.event_loop.borrow();
        let event_loop = event_loop
            .as_ref()
            .expect("Event loop has already been consumed");
        let (window, gl_config) = display_builder
            .build(event_loop, template, |mut configs| {
                configs
                    .find(|config| {
                        config.api().contains(glutin::config::Api::OPENGL)
                            && config.depth_size() >= 24
                            && config.display().supported_features().contains(
                                DisplayFeatures::CONTEXT_ROBUSTNESS
                                    | DisplayFeatures::FLOAT_PIXEL_FORMAT
                                    | DisplayFeatures::SRGB_FRAMEBUFFERS,
                            )
                    })
                    .unwrap()
            })
            .map_err(WinitError::from_dyn_error)?;
        let window = window.unwrap();
        let raw_window_handle = Some(window.raw_window_handle());
        let display = gl_config.display();
        let context_attributes = ContextAttributesBuilder::new()
            .with_debug(cfg!(debug_assertions))
            .with_profile(glutin::context::GlProfile::Core)
            .with_robustness(glutin::context::Robustness::RobustLoseContextOnReset)
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .build(raw_window_handle);
        let gl_context = unsafe { display.create_context(&gl_config, &context_attributes)? };
        Ok(Arc::new(OpenGLWindow::new(window, gl_config, gl_context)?))
    }

    fn run(
        self: Arc<Self>,
        runner: impl 'static + Fn() -> Result<bool, Self::Err>,
    ) -> Result<i32, Self::Err> {
        let mut event_loop = self
            .event_loop
            .take()
            .expect("Event loop has already been consumed");
        let ret = event_loop.run_return(|_, _, control_flow| {
            if !runner().unwrap() {
                control_flow.set_exit();
            } else {
                control_flow.set_wait_timeout(Duration::from_nanos(16_666_667));
            }
        });
        Ok(ret)
    }
}
