use eyre::Result;
use glam::Vec2;
use hecs::{EntityBuilder, World};

use assets::object::ObjectBundle;
use rose_core::transform::{Transform, TransformExt};
use rose_platform::events::WindowEvent;
use rose_platform::{Application, PhysicalSize, RenderContext};

use crate::assets::mesh::MeshAsset;
use crate::components::{Active, PanOrbitCameraBundle};
use crate::systems::{assets::AssetSystem, render::RenderSystem};
use crate::systems::{InputSystem, PanOrbitSystem};

mod assets;
pub mod components;
mod systems;

struct Sandbox {
    world: World,
    input_system: InputSystem,
    assets_system: AssetSystem,
    render_system: RenderSystem,
    pan_orbit_system: PanOrbitSystem,
    // ui_system: UiSystem,
}

impl Application for Sandbox {
    fn new(size: PhysicalSize<f32>, scale_factor: f64) -> Result<Self> {
        let logical_size = size.to_logical(scale_factor);
        let size = Vec2::from_array(size.into()).as_uvec2();
        let render_system = RenderSystem::new(size)?;

        let mut world = World::new();
        let assets_system = AssetSystem::new("assets")?;
        assets_system
            .assets
            .get_or_insert("prim:sphere", MeshAsset::uv_sphere(1., 24, 48));

        world.spawn(ObjectBundle::from_asset_cache(
            assets_system.assets.as_any_cache(),
            Transform::default(),
            "objects.moon",
        )?);
        world.spawn(EntityBuilder::new().add_bundle(PanOrbitCameraBundle::default()).add(Active).build());

        Ok(Self {
            world,
            input_system: InputSystem::default(),
            assets_system,
            render_system,
            pan_orbit_system: PanOrbitSystem::new(logical_size),
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>, scale_factor: f64) -> Result<()> {
        self.render_system.resize(size)?;
        self.pan_orbit_system.set_window_size(size.to_logical(scale_factor));
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
        self.pan_orbit_system.on_frame(&self.input_system.input, &mut self.world);

        self.render_system.on_frame(&mut self.world)?;
        self.input_system.on_frame();
        Ok(())
    }
}

fn main() -> Result<()> {
    rose_platform::run::<Sandbox>("Sandbox")
}
