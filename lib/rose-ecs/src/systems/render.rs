use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    rc::Rc,
    time::Duration,
};

use assets_manager::{AnyCache, BoxedError, Compound, Handle, SharedString};
use dashmap::DashMap;
use eyre::Result;
use glam::{UVec2, Vec2, Vec3};
use hecs::World;

use rose_core::{
    camera::Camera,
    light::{GpuLight, Light},
    transform::{Transform, TransformExt},
    utils::thread_guard::ThreadGuard,
};
use rose_platform::PhysicalSize;
use rose_renderer::{material::MaterialInstance, DrawMaterial, Mesh, Renderer};

use crate::{systems::hierarchy::GlobalTransform, assets::*, components::{Light as LightComponent, *}};

pub struct CustomMaterial<M>(ThreadGuard<Rc<M>>);

impl<M> CustomMaterial<M> {
    pub fn new(inner: M) -> Self {
        Self(ThreadGuard::new(Rc::new(inner)))
    }
}

impl<M: 'static> Compound for CustomMaterial<M> {
    fn load(_cache: AnyCache, _id: &SharedString) -> std::result::Result<Self, BoxedError> {
        Err(eyre::eyre!("Cannot load a CustomMaterial from assets, it must be provided").into())
    }
}

pub struct RenderSystem {
    pub clear_color: Vec3,
    pub camera: Camera,
    pub renderer: ThreadGuard<Renderer>,
    meshes_map: DashMap<SharedString, ThreadGuard<Rc<Mesh>>>,
    materials_map: DashMap<SharedString, ThreadGuard<Rc<MaterialInstance>>>,
    custom_materials_query: Vec<&'static (dyn Send + Sync + Fn(&mut Self, &World))>,
    lights_hash: u64,
}

impl RenderSystem {
    pub fn update_from_active_camera(&mut self, world: &World) {
        let mut q = world
            .query::<(&GlobalTransform, &CameraParams)>()
            .with::<&Active>()
            .without::<&Inactive>();
        let Some((_, (tr, camera))) = q.iter().next() else {
            tracing::warn!("No active camera. Make sure you have a camera set up using the CameraBundle, or by having GlobalTransform, CameraParams and the Active components on the entity.");
            return;
        };
        self.camera.projection.zrange = camera.zrange.clone();
        self.camera.projection.fovy = camera.fovy;
        self.camera.transform = tr.into();
    }

    pub fn default_material_handle(&self, cache: AnyCache<'static>) -> Handle<'static, Material> {
        cache.get_or_insert(
            "prim:material:default",
            Material {
                transparent: false,
                color: None,
                color_factor: Vec3::splat(0.5),
                normal: None,
                normal_amount: 1.,
                rough_metal: None,
                rough_metal_factor: Vec2::ONE,
                emission: None,
                emission_factor: Vec3::ZERO,
            },
        )
    }

    pub fn primitive_cube(&self, cache: AnyCache<'static>) -> Handle<'static, MeshAsset> {
        cache.get_or_insert("prim:cube", MeshAsset::cube())
    }

    pub fn primitive_sphere(&self, cache: AnyCache<'static>) -> Handle<'static, MeshAsset> {
        cache.get_or_insert("prim:sphere", MeshAsset::uv_sphere(1., 24, 48))
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<()> {
        let sizef = size.cast();
        self.camera.projection.width = sizef.width;
        self.camera.projection.height = sizef.height;
        self.renderer.resize(UVec2::from_array(size.into()))?;
        Ok(())
    }

    pub fn new(size: UVec2) -> Result<Self> {
        let base_dir = std::env::var("CARGO_PROJECT_DIR")
            .map(PathBuf::from)
            .or_else(|_| std::env::current_dir())
            .unwrap();
        tracing::info!("Base resources directory: {}", base_dir.display());
        let renderer = Renderer::new(size, &base_dir)?;
        Ok(Self {
            clear_color: Vec3::ZERO,
            camera: Camera::default(),
            renderer: ThreadGuard::new(renderer),
            meshes_map: DashMap::new(),
            materials_map: DashMap::new(),
            custom_materials_query: vec![],
            lights_hash: DefaultHasher::new().finish(),
        })
    }

    pub fn register_custom_material<M: 'static + DrawMaterial>(&mut self) -> &mut Self {
        self.custom_materials_query
            .push(&Self::submit_meshes_custom::<M>);
        self
    }

    pub fn on_frame(&mut self, dt: Duration, world: &World) -> Result<()> {
        self.handle_mesh_assets(world)?;
        self.handle_material_assets(world)?;
        self.handle_lights(world)?;

        self.renderer.begin_render(&self.camera)?;
        self.submit_meshes(world);
        for custom in self.custom_materials_query.clone() {
            (custom)(self, world);
        }
        self.renderer.flush(dt, self.clear_color)?;
        Ok(())
    }

    fn submit_meshes(&mut self, world: &World) {
        for (_, (mesh_handle, material_handle, transform)) in world
            .query::<(&Handle<MeshAsset>, &Handle<Material>, &GlobalTransform)>()
            .iter()
        {
            let transform = transform.into();
            tracing::trace!(message="Submitting mesh", mesh=%mesh_handle.id(), material=%material_handle.id());
            let mesh = self.meshes_map.get(mesh_handle.id()).unwrap();
            let material = self.materials_map.get(material_handle.id()).unwrap();
            self.renderer.submit_mesh_standard(
                Rc::clone(&material),
                Rc::clone(&mesh).transformed(transform),
            );
        }
    }

    fn submit_meshes_custom<M: DrawMaterial>(&mut self, world: &World) {
        for (_, (transform, material_handle, mesh_handle)) in world
            .query::<(
                &GlobalTransform,
                &Handle<CustomMaterial<M>>,
                &Handle<MeshAsset>,
            )>()
            .iter()
        {
            let transform = transform.into();
            tracing::trace!(message="Submitting mesh (custom material)", mesh=%mesh_handle.id(), material=%material_handle.id(), mat_name=%std::any::type_name::<M>());
            let material = Rc::clone(&material_handle.read().0);
            let mesh = self.meshes_map.get(mesh_handle.id()).unwrap();
            self.renderer
                .submit_mesh(material, Rc::clone(&mesh).transformed(transform));
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
                let color_slot = if let Some(color) = &mat.color {
                    Some(color.create_texture_rgb()?)
                } else {
                    None
                };
                let normal_map = if let Some(normal) = &mat.normal {
                    Some(normal.create_texture_rgb()?)
                } else {
                    None
                };
                let rough_metal = if let Some(rough_metal) = &mat.rough_metal {
                    Some(rough_metal.create_texture_rg()?)
                } else {
                    None
                };
                let emission = if let Some(emission) = &mat.emission {
                    Some(emission.create_texture_rgb()?)
                } else {
                    None
                };
                let mut inst =
                    MaterialInstance::create(color_slot, normal_map, rough_metal, emission)?;
                inst.update_uniforms(|uniforms| {
                    uniforms.color_factor = mat.color_factor;
                    uniforms.normal_amount = mat.normal_amount;
                    uniforms.rough_metal_factor = mat.rough_metal_factor;
                    uniforms.emission_factor = mat.emission_factor;
                })?;
                self.materials_map
                    .insert(handle.id().clone(), ThreadGuard::new(Rc::new(inst)));
            }
        }
        Ok(())
    }

    fn handle_lights(&mut self, world: &World) -> Result<()> {
        let light_hash = self.hash_lights(world);
        if light_hash != self.lights_hash {
            tracing::info!(message="Rebuilding lights", hash=%light_hash);
            self.lights_hash = light_hash;
            let new_lights = self
                .iter_active_lights(world)
                .into_iter()
                .inspect(|(transform, light)| {
                    tracing::debug!(message = "Light", ?transform, ?light)
                })
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
                .set_light_buffer(GpuLight::create_buffer(new_lights)?);
        }
        Ok(())
    }

    fn hash_lights(&self, world: &World) -> u64 {
        let mut hasher = DefaultHasher::new();
        for (transform, light) in self.iter_active_lights(world) {
            transform.hash(&mut hasher);
            light.hash(&mut hasher);
        }
        hasher.finish()
    }

    fn iter_active_lights(&self, world: &World) -> Vec<(Transform, LightComponent)> {
        let mut query = world
            .query::<(&GlobalTransform, &LightComponent)>()
            .with::<&Active>()
            .without::<&Inactive>();
        query.iter().map(|(_, (t, l))| (t.into(), *l)).collect()
    }
}
