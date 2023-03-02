use eyre::{ContextCompat, Result};
use glam::{Vec2, Vec3};
use hecs::{EntityBuilder, World};

use assets::object::ObjectBundle;
use rose_core::transform::{Transform, TransformExt};
use rose_platform::{
    events::WindowEvent, Application, LogicalSize, PhysicalSize, RenderContext, WindowBuilder,
};
use systems::camera::PanOrbitSystem;
use systems::input::InputSystem;

use crate::assets::scene::{NamedObject, Scene};
use crate::systems::scene::SceneSystem;
use crate::{
    assets::mesh::MeshAsset,
    components::{Active, Light, LightBundle, LightKind, PanOrbitCameraBundle},
    systems::{assets::AssetSystem, render::RenderSystem},
};

mod assets;
pub mod components;
mod systems;

struct Sandbox {
    world: World,
    input_system: InputSystem,
    assets_system: AssetSystem,
    scene_system: SceneSystem,
    render_system: RenderSystem,
    pan_orbit_system: PanOrbitSystem,
    // ui_system: UiSystem,
}

impl Application for Sandbox {
    fn window_features(wb: WindowBuilder) -> WindowBuilder {
        wb.with_inner_size(LogicalSize::new(1600, 900))
    }

    fn new(size: PhysicalSize<f32>, scale_factor: f64) -> Result<Self> {
        let logical_size = size.to_logical(scale_factor);
        let size = Vec2::from_array(size.into()).as_uvec2();
        let render_system = RenderSystem::new(size)?;

        let mut world = World::new();
        let assets_system = AssetSystem::new("assets")?;
        assets_system
            .assets
            .get_or_insert("prim:sphere", MeshAsset::uv_sphere(1., 24, 48));

        if let Some(arg) = std::env::args().skip(1).next() {
            world.spawn((assets_system.assets.load::<Scene>(&arg)?,));
        }

        let scene_system = SceneSystem::default();
        //
        // scene_system.on_frame(assets_system.assets.as_any_cache(), &mut world);
        // let scene = SceneSystem::save_world_as_scene(&world).context("Could not find the required data for a scene to be saved")?;
        // let scene_data = toml_edit::ser::to_string(&scene)?;
        // std::fs::write("suzanne.toml", scene_data)?;

        Ok(Self {
            world,
            input_system: InputSystem::default(),
            assets_system,
            scene_system,
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
        self.assets_system.on_frame();

        // let rot_quat = Quat::from_rotation_y(ctx.dt.as_secs_f32());
        // for (_, transform) in self
        //     .world
        //     .query::<&mut Transform>()
        //     .with::<&CameraParams>()
        //     .into_iter()
        // {
        //     transform.rotation *= rot_quat;
        // }
        self.scene_system
            .on_frame(self.assets_system.assets.as_any_cache(), &mut self.world);
        self.pan_orbit_system
            .on_frame(&self.input_system.input, &mut self.world);

        self.render_system.on_frame(&mut self.world)?;
        self.input_system.on_frame();
        Ok(())
    }
}

fn main() -> Result<()> {
    rose_platform::run::<Sandbox>("Sandbox")
}
