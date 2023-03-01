use std::{
    collections::HashMap,
    rc::Rc,
    time::Instant
};

use assets_manager::{Handle, SharedString};
use eyre::Result;
use glam::{UVec2, Vec3};
use hecs::World;

use rose_core::{
    camera::Camera,
    light::{GpuLight, Light},
    transform::{Transform, TransformExt},
    utils::thread_guard::ThreadGuard
};
use rose_platform::PhysicalSize;
use rose_renderer::{
    material::{
        MaterialInstance,
        TextureSlot as MaterialSlot
    },
    Mesh,
    Renderer
};
use violette::{
    framebuffer::{ClearBuffer, Framebuffer},
    texture::Texture
};

use crate::{
    assets::mesh::MeshAsset,
    components::{Active, CameraParams, Inactive}
};
use crate::assets::material::{Material, TextureSlot};

pub struct RenderSystem {
    pub clear_color: Vec3,
    pub camera: Camera,
    last_frame: Instant,
    renderer: ThreadGuard<Renderer>,
    meshes_map: HashMap<SharedString, ThreadGuard<Rc<Mesh>>>,
    materials_map: HashMap<SharedString, ThreadGuard<Rc<rose_renderer::material::MaterialInstance>>>,
    default_material_instance: ThreadGuard<Rc<MaterialInstance>>,
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
    pub fn new(size: UVec2) -> eyre::Result<Self> {
        let mut renderer = Renderer::new(size)?;
        renderer.set_light_buffer(GpuLight::create_buffer([
            Light::Ambient {
                color: Vec3::splat(0.1),
            },
            Light::Directional {
                dir: Vec3::ONE.normalize(),
                color: Vec3::ONE,
            },
        ])?);
        let default_material_instance = MaterialInstance::create([0.5; 3], None, [0.2, 0.])?;
        Ok(Self {
            clear_color: Vec3::ZERO,
            camera: Camera::default(),
            renderer: ThreadGuard::new(renderer),
            last_frame: Instant::now(),
            meshes_map: HashMap::new(),
            materials_map: HashMap::new(),
            default_material_instance: ThreadGuard::new(Rc::new(default_material_instance)),
        })
    }

    pub fn run(&mut self, world: &mut World) -> Result<()> {
        self.handle_mesh_assets(world)?;
        let has_camera = self.update_camera(world);
        if !has_camera {
            tracing::warn!("No camera found to render with. Add an entity with the `Transform` and `CameraProps`, and make it active by also adding the `Active` component.");
            Framebuffer::backbuffer().do_clear(ClearBuffer::COLOR);
            return Ok(());
        }

        self.renderer.begin_render(self.clear_color.extend(1.))?;
        self.submit_meshes(world);
        self.renderer
            .flush(&self.camera, self.last_frame.elapsed())?;
        self.last_frame = Instant::now();
        Ok(())
    }

    fn submit_meshes(&mut self, world: &mut World) {
        for (_, (mesh_handle, transform)) in
        world.query::<(&Handle<MeshAsset>, &Transform)>().iter()
        {
            let mesh = &self.meshes_map[mesh_handle.id()];
            self.renderer.submit_mesh(
                Rc::downgrade(&*self.default_material_instance),
                Rc::downgrade(&*mesh).transformed(*transform),
            );
        }
    }

    fn update_camera(&mut self, world: &mut World) -> bool {
        let mut q = world
            .query::<(&Transform, &CameraParams)>()
            .with::<&Active>()
            .without::<&Inactive>();
        let has_camera = if let Some((_, (transform, cam_params))) = q.iter().next() {
            self.camera.transform.clone_from(transform);
            self.camera.projection.fovy = cam_params.fovy;
            self.camera.projection.zrange = cam_params.zrange.clone();
            true
        } else {
            false
        };
        has_camera
    }

    fn handle_mesh_assets(&mut self, world: &mut World) -> Result<()> {
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

    fn handle_material_assets(&mut self, world: &mut World) -> Result<()> {
        for (_, handle) in world.query::<&Handle<Material>>().iter() {
            if handle.reloaded_global() || !self.materials_map.contains_key(handle.id()) {
                let mat = handle.read();
                let inst = MaterialInstance::create(
                    into_material_slot3(&mat.color)?,
                    if let Some(normal) = &mat.normal {
                        Some(Texture::from_image(normal.to_rgb32f())?)
                    } else {None},
                    into_material_slot2(&mat.rough_metal)?,
                )?;
                self.materials_map.insert(handle.id().clone(), ThreadGuard::new(Rc::new(inst)));
            }
        }
        Ok(())
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
            let storage = img.chunks_exact(3).flat_map(|s| [s[0], s[1]]).collect::<Vec<_>>();
            MaterialSlot::Texture(Texture::from_2d_pixels(img.width().try_into().unwrap(), &storage)?)
        }
    })
}