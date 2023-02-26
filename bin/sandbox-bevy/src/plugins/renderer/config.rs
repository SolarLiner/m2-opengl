use std::ops;
use glutin::config::{Api, Config, ConfigTemplateBuilder};
use bevy::prelude::{FromWorld, Windows, World};
use winit::event_loop::EventLoop;
use glutin::display::{GetGlDisplay, GlDisplay};
use crate::plugins::renderer::display::OpenGlDisplay;

pub struct OpenGlConfig(Config);

impl ops::Deref for OpenGlConfig {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromWorld for OpenGlConfig {
    fn from_world(world: &mut World) -> Self {
        world.init_non_send_resource::<OpenGlDisplay>();
        let display = world.non_send_resource::<OpenGlDisplay>();
        let windows = world.resource::<Windows>();
        let window = windows.primary();
        let template = ConfigTemplateBuilder::new()
            .compatible_with_native_window(window.raw_handle().unwrap().window_handle)
            .with_alpha_size(8)
            .with_depth_size(24)
            .with_api(Api::OPENGL);
        let config = unsafe { display.find_configs(template.build()) }.unwrap().next().unwrap();
        Self(config)
    }
}
