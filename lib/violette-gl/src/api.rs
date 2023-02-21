use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cgmath::num_traits;
use crevice::std140::AsStd140;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::{DisplayFeatures, GetGlDisplay};
use glutin::{
    config::{Config, ConfigTemplateBuilder},
    context::NotCurrentContext,
    display::Display,
    prelude::*,
};
use glutin_winit::DisplayBuilder;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use raw_window_handle::HasRawWindowHandle;
use thiserror::Error;
use winit::window::Fullscreen;
use winit::{dpi::LogicalSize, event_loop::EventLoop, window::WindowBuilder};

use violette::{Api, WindowDesc};
use crate::context::OpenGLContext;
use crate::window::OpenGLWindow;

#[derive(Debug, Copy, Clone, Error, FromPrimitive)]
#[repr(u32)]
pub enum OpenGLError {
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

impl OpenGLError {
    pub fn current_error() -> Option<Self> {
        let error = unsafe { gl::GetError() };
        if error != gl::NO_ERROR {
            Some(OpenGLError::from_u32(error).unwrap_or(OpenGLError::UnknownError))
        } else {
            None
        }
    }

    pub fn guard() -> Result<(), Self> {
        if let Some(err) = Self::current_error() {
            Err(err)
        } else {
            Ok(())
        }
    }
}

pub struct OpenGLApi {
    event_loop: EventLoop<()>,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("OpenGL error: {0}")]
    OpenGL(#[from] OpenGLError),
    #[error("Glutin context error: {0}")]
    Glutin(#[from] glutin::error::Error),
    #[error("Windowing error: {0}")]
    Winit(#[from] Box<dyn Error>),
}

impl Api for OpenGLApi {
    type Err = ApiError;
    type Buffer<T: AsStd140> = ();
    type GraphicsContext = ();
    type Window = OpenGLWindow;

    fn create_window(self: Arc<Self>, desc: WindowDesc) -> Result<Arc<Self::Window>, Self::Err> {
        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(true);
        let display_builder = DisplayBuilder::new().with_window_builder(Some(
            WindowBuilder::new()
                .with_title(desc.title)
                .with_inner_size(LogicalSize::new(desc.logical_size.x, desc.logical_size.y))
                .with_fullscreen(desc.fullscreen.then_some(Fullscreen::Borderless(None))),
        ));
        let (window, gl_config) =
            display_builder.build(&self.event_loop, template, |mut configs| {
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
            })?;
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

    fn create_graphics_context(
        self: Arc<Self>,
        window: Arc<Self::Window>,
    ) -> Result<Self::GraphicsContext, Self::Err> {
        static LOADED: AtomicBool = AtomicBool::new(false);
        let context = window.activate_context()?;
        if !LOADED.fetch_update(Ordering::Acquire, Ordering::Release, |_| true).unwrap() {
            gl::load_with(|sym| window.get_proc_address(sym.as_c_str()).cast());
        }
        Ok(OpenGLContext::instance())
    }
}
