use std::ops;
use bevy::prelude::{FromWorld, Resource, Windows, World};
use winit::event_loop::EventLoop;
use glutin::display::{Display, DisplayApiPreference};
use crate::plugins::renderer::config::OpenGlConfig;

#[derive(Resource)]
pub struct OpenGlDisplay(Display);

impl ops::Deref for OpenGlDisplay {
    type Target = Display;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromWorld for OpenGlDisplay {
    fn from_world(world: &mut World) -> Self {
        let windows = world.non_send_resource::<Windows>();
        let window = windows.primary();
        let raw_handle = window.raw_handle().unwrap();
        #[cfg(target_os = "windows")]
        let preference = DisplayApiPreference::WglThenEgl(Some(raw_handle.window_handle));
        #[cfg(target_os = "linux")]
        let preference = DisplayApiPreference::EglThenGlx(unix::register_xlib_error_hook);
        #[cfg(target_os = "macos")]
        let preference = todo!();
        let display = unsafe { Display::new(raw_handle.display_handle, preference).unwrap() };
        Self(display)
    }
}
