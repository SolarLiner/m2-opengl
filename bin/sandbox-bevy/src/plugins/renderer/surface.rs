use std::ops;

use bevy::prelude::{FromWorld, Windows, World};
use glutin::{
    display::{GetGlDisplay, GlDisplay},
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};

use crate::plugins::renderer::config::OpenGlConfig;
use crate::plugins::renderer::display::OpenGlDisplay;

pub struct RenderTargetSurface(Surface<WindowSurface>);

impl ops::Deref for RenderTargetSurface {
    type Target = Surface<WindowSurface>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromWorld for RenderTargetSurface {
    fn from_world(world: &mut World) -> Self {
        world.init_non_send_resource::<OpenGlConfig>();
        let windows = world.resource::<Windows>();
        let window = windows.primary();
        let display = world.non_send_resource::<OpenGlDisplay>();
        let config = world.non_send_resource::<OpenGlConfig>();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window.raw_handle().unwrap().window_handle,
            window.physical_width().try_into().unwrap(),
            window.physical_height().try_into().unwrap(),
        );
        let surface = unsafe { display.create_window_surface(&config, &attrs) }.unwrap();
        Self(surface)
    }
}
