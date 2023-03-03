use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    rc::Rc,
    time::Instant,
};

use assets_manager::{Handle, SharedString};
use dashmap::DashMap;
use eyre::Result;
use glam::{UVec2, Vec3};
use hecs::World;

use rose_core::{
    camera::Camera,
    light::GpuLight,
    light::Light,
    transform::{Transform, TransformExt},
    utils::thread_guard::ThreadGuard,
};
use rose_platform::PhysicalSize;
use rose_renderer::{
    material::{MaterialInstance, TextureSlot as MaterialSlot},
    Mesh, Renderer,
};
use violette::texture::Texture;

use crate::{
    assets::{
        material::{Material, TextureSlot},
        mesh::MeshAsset,
    },
    components::{Active, Inactive, Light as LightComponent, LightKind},
};

pub struct RenderSystem {
    pub clear_color: Vec3,
    pub camera: Camera,
    last_frame: Instant,
    renderer: ThreadGuard<Renderer>,
    meshes_map: DashMap<SharedString, ThreadGuard<Rc<Mesh>>>,
    materials_map: DashMap<SharedString, ThreadGuard<Rc<MaterialInstance>>>,
    default_material_instance: ThreadGuard<Rc<MaterialInstance>>,
    lights_hash: u64,
}

impl RenderSystem {
    pub fn resize(&mut self, size: PhysicalSize<u32>) -> eyre::Result<()> {
        let sizef = size.cast();
        self.camera.projection.width = sizef.width;
        self.camera.projection.height = sizef.height;
        self.renderer.resize(UVec2::from_array(size.into()))?;
        Ok(())
    }
}

impl RenderSystem {
    pub fn new(size: UVec2) -> Result<Self> {
        let default_material_instance = MaterialInstance::create([0.5; 3], None, [0.2, 0.])?;
        Ok(Self {
            clear_color: Vec3::ZERO,
            camera: Camera::default(),
            renderer: ThreadGuard::new(Renderer::new(size)?),
            last_frame: Instant::now(),
            meshes_map: DashMap::new(),
            materials_map: DashMap::new(),
            default_material_instance: ThreadGuard::new(Rc::new(default_material_instance)),
            lights_hash: DefaultHasher::new().finish(),
        })
    }

    pub fn on_frame(&mut self, world: &World) -> Result<()> {
        let (ra, rb) = rayon::join(
            || self.handle_mesh_assets(world),
            || self.handle_material_assets(world),
        );
        ra?;
        rb?;
        let light_hash = self.hash_lights(world);
        if light_hash != self.lights_hash {
            tracing::info!(message="Rebuilding lights", hash=%light_hash);
            self.lights_hash = light_hash;
            let new_lights =
                self.iter_active_lights(world)
                    .into_iter()
                    .map(|(transform, light)| {
                        let color = light.power * light.color;
                        match light.kind {
                            LightKind::Directional => Light::Directional {
                                color,
                                dir: transform.rotation.mul_vec3(Vec3::NEG_Z),
                            },
                            LightKind::Point => Light::Point {
                                color,
                                position: transform.position,
                            },
                            LightKind::Ambient => Light::Ambient { color },
                        }
                    });
            self.renderer
                .set_light_buffer(GpuLight::create_buffer(new_lights)?)
        }

        self.renderer.begin_render(self.clear_color.extend(1.))?;
        self.submit_meshes(world);
        self.renderer
            .flush(&self.camera, self.last_frame.elapsed())?;
        self.last_frame = Instant::now();
        Ok(())
    }

    fn submit_meshes(&mut self, world: &World) {
        for (_, (mesh_handle, material_handle, transform)) in world
            .query::<(&Handle<MeshAsset>, Option<&Handle<Material>>, &Transform)>()
            .iter()
        {
            tracing::trace!(message="Submitting mesh", mesh=%mesh_handle.id(), material=%material_handle.map(|h| h.id().clone()).unwrap_or("<none>".into()));
            let mesh = self.meshes_map.get(mesh_handle.id()).unwrap();
            let material = if let Some(handle) = material_handle {
                self.materials_map.get(handle.id()).unwrap().clone()
            } else {
                self.default_material_instance.clone()
            };
            self.renderer.submit_mesh(
                Rc::downgrade(&*material),
                Rc::downgrade(&*mesh).transformed(*transform),
            );
        }
    }

    fn handle_mesh_assets(&self, world: &World) -> Result<()> {
        for (_, handle) in world.query::<&Handle<MeshAsset>>().iter() {
            if handle.reloaded_global() || !self.meshes_map.contains_key(handle.id()) {
                let mesh = handle.read();
                tracing::info!(message="Loading mesh", handle=%handle.id());
                self.meshes_map.insert(
                    handle.id().clone(),
                    ThreadGuard::new(Rc::new(Mesh::new(
                        mesh.vertices.iter().copied(),
                        mesh.indices.iter().copied(),
                    )?)),
                );
            }
        }
        Ok(())
    }

    fn handle_material_assets(&self, world: &World) -> Result<()> {
        for (_, handle) in world.query::<&Handle<Material>>().iter() {
            if handle.reloaded_global() || !self.materials_map.contains_key(handle.id()) {
                tracing::info!(message="Loading material", handle=%handle.id());
                let mat = handle.read();
                let inst = MaterialInstance::create(
                    into_material_slot3(&mat.color)?,
                    if let Some(normal) = &mat.normal {
                        Some(Texture::from_image(normal.to_rgb32f())?)
                    } else {
                        None
                    },
                    into_material_slot2(&mat.rough_metal)?,
                )?
                .with_normal_amount(mat.normal_amount);
                self.materials_map
                    .insert(handle.id().clone(), ThreadGuard::new(Rc::new(inst)));
            }
        }
        Ok(())
    }

    fn hash_lights(&self, world: &World) -> u64 {
        let mut hasher = DefaultHasher::new();
        for (transform, light) in self.iter_active_lights(world) {
            transform.hash(&mut hasher);
            light.hash(&mut hasher);
        }
        return hasher.finish();
    }

    fn iter_active_lights(&self, world: &World) -> Vec<(Transform, LightComponent)> {
        let mut query = world
            .query::<(&Transform, &LightComponent)>()
            .with::<&Active>()
            .without::<&Inactive>();
        query.iter().map(|(_, (t, l))| (*t, *l)).collect()
    }
}

fn into_material_slot3(slot: &TextureSlot) -> Result<MaterialSlot<3>> {
    Ok(match slot {
        TextureSlot::Color(col) => MaterialSlot::Color(col.to_array()),
        TextureSlot::Texture(img) => MaterialSlot::Texture(Texture::from_image(img.to_rgb32f())?),
    })
}

fn into_material_slot2(slot: &TextureSlot) -> Result<MaterialSlot<2>> {
    Ok(match slot {
        TextureSlot::Color(vec) => MaterialSlot::Color(vec.truncate().to_array()),
        TextureSlot::Texture(img) => {
            let img = img.to_rgb32f();
            let storage = img
                .chunks_exact(3)
                .flat_map(|s| [s[0], s[1]])
                .collect::<Vec<_>>();
            MaterialSlot::Texture(Texture::from_2d_pixels(
                img.width().try_into().unwrap(),
                &storage,
            )?)
        }
    })
}
