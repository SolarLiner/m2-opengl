use eyre::Result;
use glam::{Quat, Vec2, vec3, Vec3};
use hecs::World;
use assets::object::ObjectBundle;

use rose_core::transform::{Transform, TransformExt};
use rose_platform::{Application, PhysicalSize, RenderContext};

use crate::components::{Active, CameraParams};
use crate::{
    systems::{assets::AssetSystem, render::RenderSystem},
};
use crate::assets::mesh::MeshAsset;

mod assets;
pub mod components;
mod systems;

struct Sandbox {
    world: World,
    assets_system: AssetSystem,
    render_system: RenderSystem,
    // ui_system: UiSystem,
}

impl Application for Sandbox {
    fn new(size: PhysicalSize<f32>) -> Result<Self> {
        let size = Vec2::from_array(size.into()).as_uvec2();
        let render_system = RenderSystem::new(size)?;

        let mut world = World::new();
        let assets_system = AssetSystem::new("assets")?;
        assets_system.assets.get_or_insert("prim:sphere", MeshAsset::uv_sphere(1., 24, 48));

        world.spawn(ObjectBundle::from_asset_cache(assets_system.assets.as_any_cache(), Transform::default(), "objects.moon")?);

        // world.spawn((
        //     Transform::default(),
        //     assets_system
        //         .assets
        //         .get_or_insert("sphere", MeshAsset::uv_sphere(2., 24, 48)),
        // ));

        world.spawn((
            Transform::translation(vec3(3., 2., -3.)).looking_at(Vec3::ZERO),
            CameraParams::default(),
            Active,
        ));

        Ok(Self {
            world,
            assets_system,
            render_system,
        })
    }

    fn render(&mut self, ctx: RenderContext) -> Result<()> {
        self.assets_system.run();

        let rot_quat = Quat::from_rotation_y(ctx.dt.as_secs_f32());
        for (_, transform) in self
            .world
            .query::<&mut Transform>()
            .with::<&CameraParams>()
            .into_iter()
        {
            transform.rotation *= rot_quat;
        }

        self.render_system.run(&mut self.world)?;
        Ok(())
    }

    fn resize(&mut self, size: PhysicalSize<u32>) -> Result<()> {
        self.render_system.resize(size)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    rose_platform::run::<Sandbox>("Sandbox")
}
