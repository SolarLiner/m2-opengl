use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::time::Duration;

use assets_manager::Handle;
use eyre::Result;
use glam::UVec2;
pub use hecs as ecs;
use hecs::Component;

use input::Input;
use rose_core::camera::Camera;
use rose_core::transform::Transform;
use rose_platform::events::WindowEvent;
use rose_platform::PhysicalSize;

use crate::assets::{Material, MeshAsset};
use crate::components::{Active, CameraParams, Inactive, Light, PanOrbitCamera};
use crate::scene::Scene;
use crate::systems::{input::InputSystem, render::RenderSystem};
use crate::systems::hierarchy::HierarchicalSystem;
use crate::systems::PersistenceSystem;

pub mod assets;
pub mod components;
pub mod scene;
pub mod systems;

pub struct CoreSystems {
    pub render: RenderSystem,
    pub input: InputSystem,
    pub persistence: PersistenceSystem,
}

impl CoreSystems {
    pub fn new(size: UVec2) -> Result<Self> {
        let mut persistence = PersistenceSystem::new();
        persistence
            .register_component::<String>()
            .register_component::<Active>()
            .register_component::<Inactive>()
            .register_component::<Transform>()
            .register_component::<CameraParams>()
            .register_component::<PanOrbitCamera>()
            .register_component::<Light>()
            .register_asset::<MeshAsset>()
            .register_asset::<Material>();
        Ok(Self {
            render: RenderSystem::new(size)?,
            input: InputSystem::default(),
            persistence,
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<()> {
        self.render.resize(size)
    }

    pub fn viewport_camera(&self) -> &Camera {
        &self.render.camera
    }

    pub fn viewport_camera_mut(&mut self) -> &mut Camera {
        &mut self.render.camera
    }

    pub fn input(&self) -> &Input {
        &self.input.input
    }

    pub fn begin_frame(&mut self) {
        self.input.on_frame();
    }

    pub fn end_frame(&mut self, scene: Option<&mut Scene>, dt: Duration) -> Result<()> {
        if let Some(scene) = scene {
            scene.with_world(|world, cmd| {
                rayon::join(
                    || {
                        self.render.update_from_active_camera(world);
                    },
                    || HierarchicalSystem.update::<Transform>(world, cmd),
                );
                self.render.on_frame(dt, world)
            })?;
            scene.flush_commands();
        }
        Ok(())
    }

    pub fn on_event(&mut self, event: WindowEvent) -> bool {
        self.input.on_event(event)
    }

    pub fn load_scene(&mut self, path: impl AsRef<Path>) -> Result<Scene> {
        Scene::load(&mut self.persistence, path)
    }

    pub fn save_scene(&mut self, scene: &Scene) -> Result<()> {
        let mut ser = serde_yaml::Serializer::new(BufWriter::new(File::create(scene.path())?));
        scene.with_world(|world, _| {
            self.persistence
                .serialize_world(scene.asset_cache(), &mut ser, world)
        })?;
        Ok(())
    }
}

pub mod prelude {
    pub use assets_manager::{
        *,
        asset::{Asset, Compound},
        source::Source,
    };
    pub use hecs::*;

    pub use crate::{
        components::*,
        CoreSystems,
        scene::Scene,
        systems::{
            hierarchy::{MakeChild, MakeChildren},
            persistence::SerializableComponent,
        },
    };
}

pub trait NamedComponent: Component {
    const NAME: &'static str;
}

impl NamedComponent for Handle<'static, assets::MeshAsset> {
    const NAME: &'static str = "Mesh";
}

impl NamedComponent for Handle<'static, assets::Material> {
    const NAME: &'static str = "Material";
}

impl NamedComponent for Transform {
    const NAME: &'static str = "Transform";
}
