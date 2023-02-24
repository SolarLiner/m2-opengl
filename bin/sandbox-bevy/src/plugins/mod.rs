use bevy::{app::PluginGroupBuilder, prelude::*};

pub use renderer::{Light, Material, Mesh, OpenGlContext, RenderTargetSurface, RoseRenderer};

use crate::plugins::renderer::RendererPlugin;

mod renderer;

pub struct ShellPlugins;

impl PluginGroup for ShellPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(RendererPlugin)
    }
}
