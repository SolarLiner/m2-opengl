use std::{
    backtrace::Backtrace,
    cell::RefCell,
    error::Error,
    fmt::{
        self,
        Formatter
    },
    sync::Arc,
    time::{Duration, Instant},
};

use dashmap::DashMap;
use glutin::{
    config::ConfigTemplateBuilder,
    display::{DisplayFeatures, GetGlDisplay},
    prelude::*,
};
use glutin_winit::DisplayBuilder;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use thiserror::Error;
use winit::{
    dpi::LogicalSize,
    event::{Event, StartCause},
    event_loop::EventLoop,
    platform::run_return::EventLoopExtRunReturn,
    window::{Fullscreen, WindowBuilder, WindowId},
};

use violette_api::{api::Api, window::Window, window::WindowDesc};

use crate::{
    thread_guard::ThreadGuard,
    window::{
        OpenGLWindow,
        WindowError
    },
    Gl
};

#[derive(Debug, Copy, Clone, Error, FromPrimitive)]
#[repr(u32)]
pub enum GlErrorKind {
    #[error("Provided enum value is not valid")]
    InvalidEnum = gl::INVALID_ENUM,
    #[error("Provided value is not valid")]
    InvalidValue = gl::INVALID_VALUE,
    #[error("Invalid OpenGL operation")]
    InvalidOperation = gl::INVALID_OPERATION,
    // #[error("Stack Overflow")]
    // StackOverflow = gl::STACK_OVERFLOW,
    // #[error("Stack Underflow")]
    // StackUnderflow = gl::STACK_UNDERFLOW,
    #[error("Out of memory")]
    OutOfMemory = gl::OUT_OF_MEMORY,
    #[error("Invalid OpenGL operation on the framebuffer")]
    InvalidFramebufferOperation = gl::INVALID_FRAMEBUFFER_OPERATION,
    // #[error("Context lost")]
    // ContextLost = gl::CONTEX,
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
    pub info: Option<String>,
    pub backtrace: Backtrace,
}

impl From<GlErrorKind> for OpenGLError {
    fn from(value: GlErrorKind) -> Self {
        Self {
            kind: value,
            info: None,
            backtrace: Backtrace::capture(),
        }
    }
}

impl fmt::Display for OpenGLError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(info) = &self.info {
            write!(f, "{}: {}", self.kind, info)
        } else {
            write!(f, "Caught OpenGL error: {}", self.kind)
        }
    }
}

impl Error for OpenGLError {}

impl OpenGLError {
    pub fn with_info_log(gl: &Gl, info: impl ToString) -> Self {
        Self {
            kind: GlErrorKind::current_error(gl).unwrap_or(GlErrorKind::UnknownError),
            info: Some(info.to_string()),
            backtrace: Backtrace::capture(),
        }
    }

    pub fn guard(gl: &Gl) -> Result<(), Self> {
        if let Some(kind) = GlErrorKind::current_error(gl) {
            Err(Self {
                kind,
                info: None,
                backtrace: Backtrace::capture(),
            })
        } else {
            Ok(())
        }
    }
}

pub struct OpenGLApi {
    event_loop: ThreadGuard<RefCell<Option<EventLoop<()>>>>,
    windows: DashMap<WindowId, Arc<OpenGLWindow>>,
}

impl OpenGLApi {
    pub fn new() -> Arc<Self> {
        let event_loop = EventLoop::new();
        Arc::new(Self {
            event_loop: ThreadGuard::new(RefCell::new(Some(event_loop))),
            windows: DashMap::new(),
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
    type Window = OpenGLWindow;

    fn create_window(self: &Arc<Self>, desc: WindowDesc) -> Result<Arc<Self::Window>, Self::Err> {
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
        let window_id = window.id();
        let window = Arc::new(OpenGLWindow::new(window, gl_config)?);
        self.windows.insert(window_id, window.clone());
        Ok(window)
    }

    fn run(self: &Arc<Self>) -> Result<i32, Self::Err> {
        let mut event_loop = self
            .event_loop
            .take()
            .expect("Event loop has already been consumed");
        let start = Instant::now();
        let mut next = start + Duration::from_nanos(16_666_667);
        let ret = event_loop.run_return(move |event, _, control_flow| {
            let _span = tracing::info_span!("winit-frame").entered();
            control_flow.set_wait_until(next);
            match event {
                Event::WindowEvent { event, window_id } => {
                    let mut remove_window = false;
                    if let Some(window) = self.windows.get(&window_id) {
                        tracing::info!(message="Window event", event=?event, id=?window_id);
                        if window.on_event(event) {
                            tracing::info!(message="Close requested", id=?window_id);
                            remove_window = true;
                        }
                    } else {
                        tracing::warn!("Received event for window that doesn't exist");
                    }
                    if remove_window {
                        self.windows.remove(&window_id);
                    }
                    if self.windows.is_empty() {
                        tracing::debug!("All windows destroyed, quitting");
                        control_flow.set_exit();
                    }
                }
                Event::RedrawRequested(id) => {
                    if let Some(window) = self.windows.get(&id) {
                        let _span = tracing::info_span!("window-draw", id=?window.key()).entered();
                        tracing::debug!(message = "Draw", id=?window.key());
                        window.on_frame().unwrap();
                        control_flow.set_wait_until(next);
                        next += Duration::from_nanos(16_666_667);
                    } else {
                        tracing::warn!("Cannot redraw with unknown window id {:?}", id);
                    }
                }
                Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                    for window in self.windows.iter() {
                        tracing::debug!(message = "Request redraw", id=?window.key());
                        window.value().request_redraw();
                    }
                }
                _ => {}
            }
        });
        Ok(ret)
    }
}