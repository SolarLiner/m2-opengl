use std::{
    fmt::{self, Formatter},
    path::{Path, PathBuf},
};

use assets_manager::{AnyCache, AssetCache};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use eyre::Result;
use hecs::{CommandBuffer, EntityBuilder, World};

use rose_core::transform::Transform;

use crate::{
    assets::{
        object::{ObjectBundle, TransformDesc},
        scene::{self as assets, NamedObject, SceneDesc, Transformed},
    },
    components::{Active, CameraBundle, CameraParams, Light, LightBundle},
};
use crate::assets::scene::Named;

pub struct Scene {
    assets: &'static AssetCache,
    world: World,
    scene_path: PathBuf,
    command_queue: (Sender<CommandBuffer>, Receiver<CommandBuffer>),
}

impl Clone for Scene {
    fn clone(&self) -> Self {
        Self::load(&self.scene_path).unwrap()
    }
}

impl fmt::Debug for Scene {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scene")
            .field("path", &self.scene_path.display())
            .finish()
    }
}

impl Scene {
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        let base_dir = base_dir.as_ref();
        let assets = Box::leak(Box::new(AssetCache::new(base_dir)?));
        assets.enhance_hot_reloading();

        Ok(Self {
            assets,
            scene_path: base_dir.join("unknown.scene"),
            world: World::new(),
            command_queue: crossbeam_channel::bounded(16),
        })
    }

    pub fn load(scene_path: impl AsRef<Path>) -> Result<Self> {
        let scene_path = scene_path.as_ref();
        let base_path = scene_path.parent().unwrap();
        let id = scene_path.file_stem().unwrap().to_str().unwrap();
        let assets = Box::leak(Box::new(AssetCache::new(&base_path)?));
        assets.enhance_hot_reloading();
        let mut world = World::new();
        Self::load_scene(&mut world, assets.as_any_cache(), id)?;
        Ok(Self {
            assets,
            scene_path: scene_path.into(),
            world,
            command_queue: crossbeam_channel::bounded(16),
        })
    }

    pub fn asset_cache(&self) -> AnyCache<'static> {
        self.assets.as_any_cache()
    }

    pub fn path(&self) -> &Path {
        self.scene_path.as_path()
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

    pub fn save(&self, editor_camera: Transformed<CameraParams>) -> Result<Option<String>> {
        match Self::save_world_as_scene(&self.world, editor_camera) {
            Some(desc) => Ok(Some(toml::ser::to_string_pretty(&desc)?)),
            None => Ok(None),
        }
    }

    fn load_scene(world: &mut World, cache: AnyCache<'static>, id: &str) -> Result<CameraBundle> {
        let mut commands = CommandBuffer::new();
        let handle = cache.load::<assets::Scene>(id)?;
        let scene = handle.read();
        let editor_camera = CameraBundle {
            transform: scene.camera.transform,
            params: scene.camera.value.clone(),
            ..Default::default()
        };
        for light in &scene.lights {
            commands.spawn(
                EntityBuilder::new()
                    .add_bundle((
                        light.name.to_string(),
                        LightBundle {
                            transform: light.transform,
                            light: light.value.value,
                            ..Default::default()
                        },
                    ))
                    .add(Active)
                    .build(),
            );
        }
        for object in &scene.objects {
            // TODO: Replace with try block
            let mut loader = || {
                commands.spawn(
                    EntityBuilder::new()
                        .add(object.name.to_string())
                        .add(NamedObject {
                            object: object.name.clone(),
                        })
                        .add_bundle(ObjectBundle {
                            transform: object.transform,
                            mesh: cache.load(object.mesh.as_str())?,
                            material: cache.load(object.material.as_str())?,
                            active: Active,
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
        Ok(editor_camera)
    }

    fn save_world_as_scene(world: &World, camera: Transformed<CameraParams>) -> Option<SceneDesc> {
        let lights = world
            .query::<(Option<&String>, &Transform, &Light)>()
            .iter()
            .map(|(_, (opt_name, transform, light))| Named {
                name: opt_name
                    .cloned()
                    .unwrap_or_else(|| String::from("<Unnamed>"))
                    .into(),
                value: Transformed {
                    transform: (*transform).into(),
                    value: *light,
                },
            })
            .collect();
        let objects = world
            .query::<(Option<&String>, &Transform, &NamedObject)>()
            .iter()
            .map(|(_, (opt_name, transform, named_obj))| Named {
                name: opt_name
                    .cloned()
                    .unwrap_or_else(|| String::from("<Unnamed>"))
                    .into(),
                value: Transformed {
                    transform: TransformDesc::from(*transform),
                    value: named_obj.clone(),
                },
            })
            .collect();
        Some(SceneDesc {
            camera,
            lights,
            objects,
        })
    }
}
