use std::{
    error::Error,
    fmt,
    fmt::Formatter,
    sync::atomic::Ordering,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use atomic_float::AtomicF32;
use cgmath::Vector2;
use crossbeam_channel::{bounded as channel, Receiver, Sender};
use glutin::{config::Config, display::GetGlDisplay, prelude::*};
use once_cell::sync::OnceCell;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use thiserror::Error;
use winit::{event::WindowEvent, window::Window};

use violette_api::window::Window as ApiWindow;
use violette_input::Input;

use crate::{
    api::{OpenGLApi, OpenGLError},
    context::{ContextError, OpenGLContext},
    thread_guard::ThreadGuard,
};

#[derive(Debug, Error)]
pub enum WindowError {
    #[error("OpenGL Context error: {0}")]
    Context(#[from] ContextError),
    #[error("OpenGL error: {0}")]
    OpenGl(#[from] OpenGLError),
}

#[derive(Debug)]
struct OpenGLWindowImpl {
    window: Window,
    config: Config,
}

pub struct OpenGLWindow {
    inner_window: ThreadGuard<OpenGLWindowImpl>,
    context: OnceCell<Arc<OpenGLContext>>,
    scale_factor: AtomicF32,
    renderer: RwLock<Box<dyn Send + Sync + Fn() -> Result<(), Box<dyn Error>>>>,
    physical_size: RwLock<Vector2<u32>>,
    events_tx: Sender<WindowEvent<'static>>,
    events_rx: Receiver<WindowEvent<'static>>,
    input: RwLock<Input>,
}

impl fmt::Debug for OpenGLWindow {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenGLWindow")
            .field("inner_window", &self.inner_window)
            .field("context", &self.context)
            .field("scale_factor", &self.scale_factor.load(Ordering::Relaxed))
            .field("renderer", &"<renderer callback>")
            .field("physical_size", &self.physical_size)
            .field("input", &self.input)
            .finish_non_exhaustive()
    }
}

impl OpenGLWindow {
    pub(crate) fn config(&self) -> &Config {
        &self.inner_window.config
    }
}

unsafe impl HasRawWindowHandle for OpenGLWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.inner_window.window.raw_window_handle()
    }
}

impl ApiWindow for OpenGLWindow {
    type Api = OpenGLApi;
    type Gc = OpenGLContext;
    type Err = WindowError;
    type Input<'a> = RwLockReadGuard<'a, Input>;

    fn attach_renderer(
        &self,
        renderer: impl 'static + Send + Sync + Fn() -> Result<(), Box<dyn Error>>,
    ) {
        *self.renderer.write().unwrap() = Box::new(renderer);
    }

    fn request_redraw(&self) {
        self.inner_window.window.request_redraw();
    }

    fn vsync(&self) -> bool {
        false
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor.load(Ordering::Relaxed)
    }

    fn physical_size(&self) -> Vector2<u32> {
        *self.physical_size.read().unwrap()
    }

    fn input(&self) -> Self::Input<'_> {
        self.input.read().unwrap()
    }

    fn context(self: &Arc<Self>) -> Result<Arc<Self::Gc>, Self::Err> {
        tracing::debug!("Get context");
        Ok(self
            .context
            .get_or_try_init(|| {
                tracing::debug!("Create OpenGL context");
                Ok::<_, WindowError>(Arc::new(OpenGLContext::new(self.clone())?))
            })?
            .clone())
    }

    fn on_frame(&self) -> Result<(), Self::Err> {
        if let Some(ctx) = self.context.get() {
            ctx.make_current()?;
        } else {
            tracing::info!(
                "Skipping rendering of window {:?} as it has no context defined",
                self.inner_window.window.id()
            );
        }
        self.input.write().unwrap().new_frame();
        let renderer = self.renderer.read().unwrap();
        match renderer() {
            Ok(()) => {}
            Err(err) => {
                tracing::error!(
                    "Error during rendering of window {:?}: {}",
                    self.inner_window.window.id(),
                    err
                );
            }
        }
        Ok(())
    }

    fn on_update(&self) -> Result<(), Self::Err> {
        Ok(())
    }
}

impl OpenGLWindow {
    pub fn next_event(&self) -> Option<WindowEvent<'static>> {
        self.events_rx.try_recv().ok()
    }
}

impl OpenGLWindow {
    pub(crate) fn new(window: Window, config: Config) -> Result<Self, WindowError> {
        let inner_size = window.inner_size();
        let physical_size = Vector2::new(inner_size.width, inner_size.height);
        let scale_factor = window.scale_factor();
        let inner_window = OpenGLWindowImpl { window, config };
        let (events_tx, events_rx) = channel(32);
        Ok(Self {
            inner_window: ThreadGuard::new(inner_window),
            context: OnceCell::new(),
            physical_size: RwLock::new(physical_size),
            scale_factor: AtomicF32::new(scale_factor as _),
            input: RwLock::new(Input::default()),
            renderer: RwLock::new(Box::new(|| Ok(()))),
            events_tx,
            events_rx,
        })
    }

    pub(crate) fn on_event(&self, event: WindowEvent) -> bool {
        match event {
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor
                    .store(scale_factor as _, Ordering::Relaxed);
            }
            WindowEvent::Resized(new_size) => {
                let new_size = Vector2::new(new_size.width, new_size.height);
                *self.physical_size.write().unwrap() = new_size;
                if let Some(context) = self.context.get() {
                    context.resize(new_size);
                }
            }
            WindowEvent::CloseRequested => {
                self.inner_window.window.set_visible(false);
                return true;
            }
            event => {
                if let Some(event) = self.input.write().unwrap().update_from_event(event) {
                    tracing::trace!("Event {:?}", event);
                    // self.events_tx.send(event.to_static().unwrap()).unwrap();
                }
            }
        }
        false
    }
}
