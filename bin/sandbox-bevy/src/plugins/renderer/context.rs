use std::ops;

use bevy::prelude::*;
use glutin::{
    context::{
        ContextApi, ContextAttributesBuilder, GlProfile, PossiblyCurrentContext, Robustness, Version,
    },
    prelude::*
};

use super::{
    config::OpenGlConfig, display::OpenGlDisplay, surface::RenderTargetSurface,
};

pub struct OpenGlContext(PossiblyCurrentContext);

impl ops::Deref for OpenGlContext {
    type Target = PossiblyCurrentContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromWorld for OpenGlContext {
    fn from_world(world: &mut World) -> Self {
        world.init_non_send_resource::<OpenGlConfig>();
        world.init_non_send_resource::<OpenGlDisplay>();
        let windows = world.non_send_resource::<Windows>();
        let window = windows.primary();
        let config = world.non_send_resource::<OpenGlConfig>();
        let display = world.non_send_resource::<OpenGlDisplay>();
        let ctx_attrs = ContextAttributesBuilder::new()
            .with_debug(cfg!(debug_assertions))
            .with_profile(GlProfile::Core)
            .with_robustness(Robustness::RobustLoseContextOnReset)
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .build(Some(window.raw_handle().unwrap().window_handle));
        let not_current_context = unsafe {
            display
                .create_context(&config, &ctx_attrs)
                .expect("Cannot create OpenGL context")
        };
        let surface = &*world.non_send_resource::<RenderTargetSurface>();
        Self(not_current_context.make_current(surface).unwrap())
    }
}
