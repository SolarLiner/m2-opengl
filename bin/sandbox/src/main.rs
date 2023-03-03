use eyre::{ContextCompat, Result};
use glam::{Vec2, Vec3};

use crate::components::PanOrbitCamera;
use crate::{
    scene::Scene,
    systems::{camera::PanOrbitSystem, input::InputSystem, render::RenderSystem},
};
use rose_core::transform::TransformExt;
use rose_platform::{
    events::WindowEvent, Application, LogicalSize, PhysicalSize, RenderContext, WindowBuilder,
};

mod assets;
pub mod components;
mod scene;
mod systems;

struct Sandbox {
    editor_scene: Option<Scene>,
    active_scene: Option<Scene>,
    editor_cam_controller: PanOrbitCamera,
    input_system: InputSystem,
    render_system: RenderSystem,
    pan_orbit_system: PanOrbitSystem,
}

impl Sandbox {
    fn start_active_scene(&mut self) {
        self.stop_active_scene();
        if let Some(scene) = &self.editor_scene {
            self.active_scene.replace(scene.clone());
        }
    }

    fn stop_active_scene(&mut self) {
        self.active_scene.take();
    }
}

impl Application for Sandbox {
    fn window_features(wb: WindowBuilder) -> WindowBuilder {
        wb.with_inner_size(LogicalSize::new(1600, 900))
    }

    fn new(size: PhysicalSize<f32>, scale_factor: f64) -> Result<Self> {
        let logical_size = size.to_logical(scale_factor);
        let size = Vec2::from_array(size.into()).as_uvec2();
        let render_system = RenderSystem::new(size)?;

        let editor_scene =
            std::env::args()
                .skip(1)
                .next()
                .and_then(|id| match Scene::load("assets", &id) {
                    Ok(scene) => Some(scene),
                    Err(err) => {
                        tracing::error!("Cannot load scene: {}", err);
                        None
                    }
                });

        Ok(Self {
            editor_scene,
            active_scene: None,
            editor_cam_controller: PanOrbitCamera::default(),
            input_system: InputSystem::default(),
            render_system,
            pan_orbit_system: PanOrbitSystem::new(logical_size),
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>, scale_factor: f64) -> Result<()> {
        self.render_system.resize(size)?;
        self.pan_orbit_system
            .set_window_size(size.to_logical(scale_factor));
        Ok(())
    }

    fn interact(&mut self, event: WindowEvent) -> Result<()> {
        self.input_system.on_event(event);
        Ok(())
    }

    fn render(&mut self, ctx: RenderContext) -> Result<()> {
        if let Some(scene) = &mut self.active_scene {
            scene.on_frame();
            scene.with_world_mut(|world| {
                self.pan_orbit_system
                    .on_frame(&self.input_system.input, world)
            });
            scene.with_world(|world, _| {
                let ret = self.render_system.on_frame(world);
                self.input_system.on_frame();
                ret
            })?;
            scene.flush_commands();
        } else if let Some(scene) = &mut self.editor_scene {
            scene.on_frame();
            self.pan_orbit_system.frame_manual(
                &self.input_system.input,
                &mut self.editor_cam_controller,
                &mut self.render_system.camera.transform,
            );
            scene.with_world(|world, command_buffer| {
                let ret = self.render_system.on_frame(world);
                self.input_system.on_frame();
                ret
            })?;
            scene.flush_commands();
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    rose_platform::run::<Sandbox>("Sandbox")
}
