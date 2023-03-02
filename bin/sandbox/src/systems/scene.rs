use assets_manager::{AnyCache, ReloadWatcher, SharedString};
use egui::epaint::ahash::HashMap;
use hecs::{CommandBuffer, EntityBuilder, World};

use rose_core::transform::Transform;

use crate::assets::object::{ObjectBundle, TransformDesc};
use crate::assets::scene::{NamedObject, Scene, SceneDesc, Transformed};
use crate::components::{Active, CameraParams, Light, LightBundle, PanOrbitCameraBundle, SceneId};

#[derive(Debug, Clone, Default)]
pub struct SceneSystem {
    tracked_scenes: HashMap<SharedString, ReloadWatcher<'static>>,
}

impl SceneSystem {
    pub fn on_frame(&mut self, cache: AnyCache<'static>, world: &mut World) {
        let mut q = world.query::<&SceneId>();
        let mut commands = CommandBuffer::new();
        for (entity, scene) in q.iter() {
            let handle = match cache.load::<Scene>(scene.0.as_str()) {
                Ok(scene) => scene,
                Err(err) => {
                    tracing::error!("Cannot load scene '{}': {}", scene.0.as_str(), err);
                    continue;
                }
            };
            self.tracked_scenes
                .insert(scene.0.clone(), handle.reload_watcher());
            let scene = handle.read();
            commands.spawn(
                EntityBuilder::new()
                    .add_bundle(PanOrbitCameraBundle {
                        transform: scene.camera.transform,
                        params: scene.camera.value.clone(),
                        ..Default::default()
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
            commands.remove_one::<Scene>(entity);
        }

        std::mem::drop(q);
        commands.run_on(world);
    }

    pub fn save_world_as_scene(world: &World) -> Option<SceneDesc> {
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
