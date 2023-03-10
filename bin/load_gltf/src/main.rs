use std::path::PathBuf;

use eyre::Result;
use glam::{UVec2, Vec2, Vec3};

use rose_core::transform::Transform;
use rose_ecs::{
    prelude::*,
    systems::{hierarchy::GlobalTransform, hierarchy::HierarchicalSystem, PanOrbitSystem},
};
use rose_platform::{Application, events::WindowEvent, PhysicalSize, RenderContext};
use rose_renderer::env::EnvironmentMap;

mod load_gltf;

struct App {
    core_systems: CoreSystems,
    scene: Scene,
    pan_orbit_system: PanOrbitSystem,
}

impl Application for App {
    #[tracing::instrument]
    fn new(size: PhysicalSize<f32>, scale_factor: f64) -> Result<Self> {
        let sizeu = UVec2::from_array(size.cast::<u32>().into());
        let mut core_systems = CoreSystems::new(sizeu)?;
        core_systems
            .persistence
            .register_component::<GlobalTransform>();
        core_systems
            .render
            .renderer
            .set_environment(|reload_watcher| EnvironmentMap::load(
                "assets/textures/derelict_highway_midday_1k.exr",
                reload_watcher,
            ).unwrap());
        let scene = if let Some(name) = std::env::args().nth(1) {
            let path = PathBuf::from(name);
            let mut scene: Scene = smol::block_on(load_gltf::load_gltf_scene(&path))?;
            scene.with_world(|world, cmd| {
                // cmd.spawn(LightBundle {
                //     transform: Transform::translation(Vec3::ONE).looking_at(Vec3::ZERO),
                //     light: Light {
                //         kind: LightKind::Directional,
                //         ..Default::default()
                //     },
                //     ..Default::default()
                // });
                cmd.spawn(PanOrbitCameraBundle {
                    pan_orbit: PanOrbitCamera {
                        focus: Vec3::ZERO,
                        radius: 3.,
                        target_rotation: Vec2::ZERO,
                    },
                    ..Default::default()
                });
                HierarchicalSystem.update::<Transform>(world, cmd);
            });
            scene.flush_commands();
            scene.set_path("assets/from_gltf.scene");
            core_systems.save_scene(&scene)?;
            scene
        } else {
            eyre::bail!("Need to provide a file to open");
        };
        let pan_orbit_system = PanOrbitSystem::new(size.to_logical(scale_factor));
        Ok(Self {
            core_systems,
            scene,
            pan_orbit_system,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>, scale_factor: f64) -> Result<()> {
        self.core_systems.resize(size)?;
        self.pan_orbit_system.resize(size.to_logical(scale_factor));
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn interact(&mut self, event: WindowEvent) -> Result<()> {
        let _ = self.core_systems.on_event(event);
        Ok(())
    }

    #[cfg(never)]
    fn tick(&mut self, ctx: TickContext) -> Result<()> {
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn render(&mut self, ctx: RenderContext) -> Result<()> {
        self.core_systems.begin_frame();
        self.scene.with_world_mut(|world| {
            self.pan_orbit_system
                .on_frame(&self.core_systems.input.input, world);
        });
        self.core_systems.end_frame(Some(&mut self.scene), ctx.dt)
    }
}

fn main() -> Result<()> {
    rose_platform::run::<App>("Load GLTF")
}
