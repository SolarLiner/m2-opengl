use crate::components::Light;
use crate::{
    assets::{
        object::{ObjectBundle, TransformDesc},
        scene as assets,
        scene::{NamedObject, SceneDesc, Transformed},
    },
    components::{Active, CameraBundle, CameraParams, LightBundle},
};
use assets_manager::{AnyCache, AssetCache, SharedString};
use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError};
use eyre::Result;
use hecs::{CommandBuffer, EntityBuilder, Query, QueryBorrow, World};
use rose_core::transform::Transform;
use std::time::Duration;
use std::{
    fmt::{self, Formatter},
    path::PathBuf,
};

pub struct Scene {
    assets: &'static AssetCache,
    world: World,
    scene_id: SharedString,
    asset_base_dir: PathBuf,
    command_queue: (Sender<CommandBuffer>, Receiver<CommandBuffer>),
}

impl Clone for Scene {
    fn clone(&self) -> Self {
        Self::load(&self.asset_base_dir, &self.scene_id).unwrap()
    }
}

impl fmt::Debug for Scene {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scene")
            .field("base_dir", &self.asset_base_dir.display())
            .field("scene_id", &self.scene_id)
            .finish()
    }
}

impl Scene {
    pub fn load(base_path: impl Into<PathBuf>, id: &str) -> Result<Self> {
        let base_path = base_path.into();
        let assets = Box::leak(Box::new(AssetCache::new(&base_path)?));
        assets.enhance_hot_reloading();
        let mut world = World::new();
        Self::load_scene(&mut world, assets.as_any_cache(), id)?;
        Ok(Self {
            assets,
            asset_base_dir: base_path,
            scene_id: id.into(),
            world,
            command_queue: crossbeam_channel::bounded(16),
        })
    }

    pub fn on_frame(&self) {
        self.assets.hot_reload();
    }

    #[inline]
    pub fn with_world<R>(&self, runner: impl FnOnce(&World, &mut CommandBuffer) -> R) -> R {
        let mut command_buffer = CommandBuffer::new();
        let ret = runner(&self.world, &mut command_buffer);
        match self.command_queue.0.try_send(command_buffer) {
            Ok(_) => {}
            Err(err) => {
                tracing::error!("Cannot send command buffer: {}", err)
            }
        }
        ret
    }

    #[inline]
    pub fn with_world_mut<R>(&mut self, runner: impl FnOnce(&mut World) -> R) -> R {
        runner(&mut self.world)
    }

    pub fn flush_commands(&mut self) {
        loop {
            match self.command_queue.1.try_recv() {
                Ok(mut cmd) => {
                    cmd.run_on(&mut self.world);
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
                Err(err) => {
                    tracing::error!("Cannot receive command buffer: {}", err);
                    break;
                }
            }
        }
    }

    pub fn save(&self) -> Result<Option<String>> {
        match Self::save_world_as_scene(&self.world) {
            Some(desc) => Ok(Some(toml::ser::to_string_pretty(&desc)?)),
            None => Ok(None),
        }
    }

    fn load_scene(world: &mut World, cache: AnyCache<'static>, id: &str) -> Result<()> {
        let mut commands = CommandBuffer::new();
        let handle = cache.load::<assets::Scene>(id)?;
        let scene = handle.read();
        commands.spawn(
            EntityBuilder::new()
                .add_bundle(CameraBundle {
                    transform: scene.camera.transform,
                    params: scene.camera.value.clone(),
                })
                .add(Active)
                .build(),
        );
        for light in &scene.lights {
            commands.spawn(
                EntityBuilder::new()
                    .add_bundle(LightBundle {
                        transform: light.transform,
                        light: light.value,
                    })
                    .add(Active)
                    .build(),
            );
        }
        for object in &scene.objects {
            // TODO: Replace with try block
            let mut loader = || {
                commands.spawn(
                    EntityBuilder::new()
                        .add(NamedObject {
                            object: object.name.clone(),
                        })
                        .add_bundle(ObjectBundle {
                            transform: object.transform,
                            mesh: cache.load(object.mesh.as_str())?,
                            material: cache.load(object.material.as_str())?,
                        })
                        .build(),
                );
                // commands.spawn(ObjectBundle {
                //     transform: object.transform,
                //     mesh: cache.load(object.mesh.as_str())?,
                //     material: cache.load(object.material.as_str())?,
                // });
                Ok::<_, eyre::Report>(())
            };
            match loader() {
                Ok(_) => {}
                Err(err) => tracing::warn!("Cannot load object: {}", err),
            }
        }

        commands.run_on(world);
        Ok(())
    }

    fn save_world_as_scene(world: &World) -> Option<SceneDesc> {
        let camera = world
            .query::<(&Transform, &CameraParams)>()
            .iter()
            .next()
            .map(|(_, (transform, cam))| Transformed {
                transform: (*transform).into(),
                value: cam.clone(),
            })?;
        let lights = world
            .query::<(&Transform, &Light)>()
            .iter()
            .map(|(_, (transform, light))| Transformed {
                transform: (*transform).into(),
                value: *light,
            })
            .collect();
        let objects = world
            .query::<(&Transform, &NamedObject)>()
            .iter()
            .map(|(_, (transform, named_obj))| Transformed {
                transform: TransformDesc::from(*transform),
                value: named_obj.clone(),
            })
            .collect();
        Some(SceneDesc {
            camera,
            lights,
            objects,
        })
    }
}
