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
use crate::systems::hierarchy::{HierarchicalSystem, Parent};
use crate::systems::PersistenceSystem;
use crate::systems::{input::InputSystem, render::RenderSystem};

pub mod assets;
pub mod components;
pub mod load_gltf;
pub mod prelude;
pub mod scene;
pub mod systems;

pub struct CoreSystems {
    pub render: RenderSystem,
    pub input: InputSystem,
    pub persistence: PersistenceSystem,
    pub manual_camera_update: bool,
}

impl CoreSystems {
    pub fn new(size: UVec2) -> Result<Self> {
        let mut persistence = PersistenceSystem::new();
        persistence
            .register_component::<String>()
            .register_component::<Parent>()
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
            manual_camera_update: false,
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

    pub fn begin_frame(&mut self) {}

    pub fn end_frame(&mut self, scene: Option<&mut Scene>, dt: Duration) -> Result<()> {
        if let Some(scene) = scene {
            scene.with_world(|world, cmd| {
                HierarchicalSystem.update::<Transform>(world, cmd);
                if !self.manual_camera_update {
                    self.render.update_from_active_camera(world);
                }
                self.render.on_frame(dt, world)
            })?;
            scene.flush_commands();
        }
        self.input.on_frame();
        Ok(())
    }

    pub fn on_event<'ev>(&mut self, event: WindowEvent<'ev>) -> Option<WindowEvent<'ev>> {
        self.input.on_event(event)
    }

    pub fn load_scene(&mut self, path: impl AsRef<Path>) -> Result<Scene> {
        Scene::load(&mut self.persistence, path)
    }

    pub fn save_scene(&mut self, scene: &Scene) -> Result<()> {
        let mut ser = serde_yaml::Serializer::new(BufWriter::new(File::create(scene.path())?));
        scene.with_world(|world, _| {
            self.persistence
                .serialize_world(scene.asset_cache().as_any_cache(), &mut ser, world)
        })?;
        Ok(())
    }
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
