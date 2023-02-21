use cgmath::Vector2;
use glutin::config::Config;
use glutin::context::{NotCurrentContext, PossiblyCurrentContext};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, SurfaceAttributesBuilder, WindowSurface};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use thiserror::Error;
use winit::window::Window;
use violette as api;

#[derive(Debug, Copy, Clone, Error)]
pub enum WindowError {
    #[error("Glutin context error: {0}")]
    GlutinError(#[from] glutin::error::Error),
}

pub struct OpenGLWindow {
    window: Window,
    scale_factor: f32,
    physical_size: Vector2<f32>,
    surface: Surface<WindowSurface>,
    config: Config,
    context: NotCurrentContext,
}

unsafe impl HasRawWindowHandle for OpenGLWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.window.raw_window_handle()
    }
}

impl api::Window for OpenGLWindow {
    type Err = ();
    type Api = ();

    fn physical_size(&self) -> Vector2<f32> {
        self.physical_size
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    fn update(&self) -> Result<(), Self::Err> {
        Ok(())
    }

    fn vsync(&self) -> bool {
        false
    }

    fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

impl OpenGLWindow {
    pub(crate) fn new(window: Window, config: Config, context: NotCurrentContext) -> Result<Self, WindowError> {
        let inner_size = window.inner_size();
        let physical_size = inner_size.cast();
        let physical_size = Vector2::new(physical_size.width, physical_size.height);
        let scale_factor = window.scale_factor() as _;
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(window.raw_window_handle(), inner_size.width.try_into().unwrap(), inner_size.height.try_into().unwrap());
        let surface = unsafe { config.display().create_window_surface(&config, &attrs) }?;
        Ok(Self {
            window,
            physical_size,
            scale_factor,
            surface,
            config,
            context,
        })
    }

    pub(crate) fn activate_context(&self) -> Result<PossiblyCurrentContext, WindowError> {
        Ok(self.context.make_current(&self.surface)?)
    }
}
