use std::ffi::{c_void, CStr, CString};
use std::sync::Arc;

use cgmath::Vector2;
use glutin::{
    config::Config,
    context::{NotCurrentContext, PossiblyCurrentContext},
    display::GetGlDisplay,
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use thiserror::Error;
use winit::window::Window;

use violette_api::{api::Api, window::Window as ApiWindow};

use crate::{
    api::{OpenGLApi, OpenGLError},
    context::OpenGLContext,
    thread_guard::ThreadGuard,
};

#[derive(Debug, Error)]
pub enum WindowError {
    #[error("Glutin context error: {0}")]
    Glutin(#[from] glutin::error::Error),
    #[error("OpenGL error: {0}")]
    OpenGl(#[from] OpenGLError),
}

struct OpenGLWindowImpl {
    window: Window,
    surface: Surface<WindowSurface>,
    config: Config,
}

pub struct OpenGLWindow {
    inner_window: ThreadGuard<OpenGLWindowImpl>,
    context: Arc<ThreadGuard<PossiblyCurrentContext>>,
    scale_factor: f32,
    physical_size: Vector2<u32>,
}

unsafe impl HasRawWindowHandle for OpenGLWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.inner_window.window.raw_window_handle()
    }
}

impl ApiWindow for OpenGLWindow {
    type Api = OpenGLApi;
    type Err = WindowError;

    fn request_redraw(&self) {
        self.inner_window.window.request_redraw();
    }

    fn vsync(&self) -> bool {
        false
    }

    fn update(&self) -> Result<(), Self::Err> {
        Ok(())
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    fn physical_size(&self) -> Vector2<u32> {
        self.physical_size
    }
}

impl OpenGLWindow {
    pub(crate) fn new(
        window: Window,
        config: Config,
        context: NotCurrentContext,
    ) -> Result<Self, WindowError> {
        let inner_size = window.inner_size();
        let physical_size = Vector2::new(inner_size.width, inner_size.height);
        let scale_factor = window.scale_factor() as _;
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window.raw_window_handle(),
            inner_size.width.try_into().unwrap(),
            inner_size.height.try_into().unwrap(),
        );
        let surface = unsafe { config.display().create_window_surface(&config, &attrs) }?;
        let context = Arc::new(ThreadGuard::new(context.make_current(&surface)?));
        let inner_window = OpenGLWindowImpl {
            window,
            surface,
            config,
        };
        Ok(Self {
            inner_window: ThreadGuard::new(inner_window),
            context,
            physical_size,
            scale_factor,
        })
    }

    pub(crate) fn context(&self) -> Arc<ThreadGuard<PossiblyCurrentContext>> {
        self.context.clone()
    }

    pub(crate) fn get_proc_address(&self, sym: &str) -> *const c_void {
        let sym = CString::new(sym).unwrap();
        self.context.display().get_proc_address(sym.as_c_str())
    }
}
