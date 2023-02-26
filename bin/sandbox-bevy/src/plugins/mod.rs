use bevy::{app::PluginGroupBuilder, prelude::*};

pub use camera::*;
pub use renderer::*;
pub use ui::*;

use crate::plugins::{camera::DollyPlugin, renderer::RendererPlugin};

mod camera;
mod renderer;
mod ui;

pub struct ShellPlugins;

impl PluginGroup for ShellPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(DollyPlugin)
            .add(RendererPlugin)
            .add(UiPlugin)
    }
}
