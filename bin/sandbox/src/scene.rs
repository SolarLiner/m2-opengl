use std::sync::{Arc, Weak};

use glam::Vec2;

use rose_core::camera::{Camera, Projection};
use rose_core::transform::TransformExt;
use rose_core::{
    light::{GpuLight, Light, LightBuffer},
    material::Material,
    transform::{Transform, Transformed},
};
use rose_renderer::Mesh;

#[derive(Debug, Clone)]
pub enum Entity {
    Light(Weak<Light>),
    Object(Weak<Material>, Weak<Mesh>),
    Camera(Projection),
}

#[derive(Debug, Clone)]
pub struct Instance {
    id: u64,
    pub name: Option<String>,
    entity: Entity,
    pub transform: Transform,
}

impl Instance {
    pub fn from_entity(id: u64, entity: Transformed<Entity>) -> Self {
        Self {
            id,
            name: None,
            entity: entity.value,
            transform: entity.transform,
        }
    }

    pub fn named(&mut self, name: impl ToString) -> &mut Self {
        self.name.replace(name.to_string());
        self
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn entity(&self) -> &Entity {
        &self.entity
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct CameraId(u64);

#[derive(Debug, Clone)]
pub struct Scene {
    next_id: u64,
    active_camera_id: Option<u64>,
    light_storage: Vec<Arc<Light>>,
    mesh_storage: Vec<Arc<Mesh>>,
    material_storage: Vec<Arc<Material>>,
    instances: Vec<Instance>,
    need_relight: bool,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            active_camera_id: None,
            light_storage: vec![],
            mesh_storage: vec![],
            material_storage: vec![],
            instances: vec![],
            need_relight: true,
        }
    }

    pub fn get(&self, id: u64) -> Option<&Instance> {
        if let Ok(ix) = self.instances.binary_search_by_key(&id, |inst| inst.id) {
            Some(&self.instances[ix])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Instance> {
        if let Ok(ix) = self.instances.binary_search_by_key(&id, |inst| inst.id) {
            self.need_relight = true;
            Some(&mut self.instances[ix])
        } else {
            None
        }
    }

    pub fn camera_instance(&self, CameraId(id): CameraId) -> Option<&Instance> {
        self.get(id)
    }

    pub fn camera_instance_mut(&mut self, CameraId(id): CameraId) -> Option<&mut Instance> {
        self.get_mut(id)
    }

    pub fn instances(&self) -> impl '_ + Iterator<Item = &Instance> {
        self.instances.iter()
    }

    pub fn instances_mut(&mut self) -> impl '_ + Iterator<Item = &mut Instance> {
        // Conservatively reset lighting
        self.need_relight = true;
        self.instances.iter_mut()
    }

    pub fn objects(&self) -> impl '_ + Iterator<Item = (Weak<Material>, Transformed<Weak<Mesh>>)> {
        self.instances.iter().filter_map(|inst| match &inst.entity {
            Entity::Object(material, mesh) => {
                Some((material.clone(), mesh.clone().transformed(inst.transform)))
            }
            _ => None,
        })
    }

    pub fn resize_cameras(&mut self, size: Vec2) {
        for obj in self
            .instances
            .iter_mut()
            .filter_map(|inst| match &mut inst.entity {
                Entity::Camera(proj) => Some(proj),
                _ => None,
            })
        {
            obj.width = size.x;
            obj.height = size.y;
        }
    }

    pub fn active_camera(&self) -> Option<Camera> {
        if let Some(id) = self.active_camera_id {
            let instance = self.get(id)?;
            match &instance.entity {
                Entity::Camera(projection) => Some(Camera {
                    projection: projection.clone(),
                    transform: instance.transform,
                }),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn active_camera_id(&self) -> Option<CameraId> {
        self.active_camera_id.map(CameraId)
    }

    pub fn update_camera(&mut self, CameraId(id): CameraId, camera: &Camera) {
        if let Some(inst) = self.get_mut(id) {
            inst.transform.clone_from(&camera.transform);
            match &mut inst.entity {
                Entity::Camera(projection) => projection.clone_from(&camera.projection),
                _ => unreachable!(),
            }
        }
    }

    pub fn add_mesh(&mut self, mesh: impl Into<Arc<Mesh>>) -> Weak<Mesh> {
        let mesh = mesh.into();
        let weakref = Arc::downgrade(&mesh);
        self.mesh_storage.push(mesh);
        weakref
    }

    pub fn add_material(&mut self, material: impl Into<Arc<Material>>) -> Weak<Material> {
        let material = material.into();
        let weakref = Arc::downgrade(&material);
        self.material_storage.push(material);
        weakref
    }

    pub fn add_light(&mut self, light: impl Into<Arc<Light>>) -> Weak<Light> {
        let light = light.into();
        let weakref = Arc::downgrade(&light);
        self.light_storage.push(light);
        weakref
    }

    pub fn add_camera(&mut self, camera: Transformed<Projection>) -> CameraId {
        let id = self.next_id;
        self.instances
            .push(Instance::from_entity(id, camera.map(Entity::Camera)));
        self.next_id += 1;
        CameraId(id)
    }

    pub fn set_active_camera(&mut self, CameraId(id): CameraId) {
        let _ = self.get(id).expect("ID is non-valid");
        self.active_camera_id.replace(id);
    }

    pub fn updated_light_buffer(&mut self) -> Option<eyre::Result<LightBuffer>> {
        self.need_relight.then(|| {
            self.need_relight = false;
            GpuLight::create_buffer(self.instances.iter().filter_map(|inst| match &inst.entity {
                Entity::Light(light) => {
                    Some(light.upgrade().unwrap().with_transform(inst.transform))
                }
                _ => None,
            }))
        })
    }

    pub fn lights(&self) -> impl '_ + Iterator<Item=&Light> {
        self.light_storage.iter().map(|arc| arc.as_ref())
    }

    pub fn instance_light(&mut self, light: Transformed<Weak<Light>>) -> &mut Instance {
        let id = self.next_id;
        let ix = self.instances.len();
        self.instances
            .push(Instance::from_entity(id, light.map(Entity::Light)));
        self.need_relight = true;
        self.next_id += 1;
        &mut self.instances[ix]
    }

    pub fn remove(&mut self, id: u64) {
        let Ok(ix) = self.instances.binary_search_by_key(&id, |inst| inst.id) else {return;};
        let inst = self.instances.remove(ix);
        match inst.entity {
            Entity::Light(light) => {
                self.maybe_remove_light(&light);
                self.need_relight = true;
            }
            Entity::Object(material, mesh) => {
                self.maybe_remove_material(&material);
                self.maybe_remove_mesh(&mesh);
            }
            Entity::Camera(_) => {}
        }
    }

    pub fn instance_object(
        &mut self,
        material: Weak<Material>,
        mesh: Transformed<Weak<Mesh>>,
    ) -> &mut Instance {
        let id = self.next_id;
        let ix = self.instances.len();
        self.instances.push(Instance::from_entity(
            id,
            mesh.map(|mesh| Entity::Object(material, mesh)),
        ));
        self.next_id += 1;
        &mut self.instances[ix]
    }

    pub fn merge(mut self, other: Self) -> Self {
        for instance in other.instances {
            match instance.entity {
                Entity::Light(light) => {
                    let light = self.add_light(light.upgrade().unwrap());
                    self.instance_light(light.transformed(instance.transform));
                }
                Entity::Object(material, mesh) => {
                    let material = self.add_material(material.upgrade().unwrap());
                    let mesh = self.add_mesh(mesh.upgrade().unwrap());
                    self.instance_object(material, mesh.transformed(instance.transform));
                }
                Entity::Camera(proj) => {
                    let id = self.add_camera(proj.transformed(instance.transform));
                    if other.active_camera_id == Some(instance.id) {
                        self.active_camera_id = Some(id.0);
                    }
                }
            }
        }
        self
    }

    fn maybe_remove_light(&mut self, light: &Weak<Light>) {
        if light.weak_count() == 1 {
            let Some(ix) = self.light_storage.iter().position(|l| Arc::ptr_eq(l, &light.upgrade().unwrap())) else { return; };
            self.light_storage.remove(ix);
        }
    }

    fn maybe_remove_mesh(&mut self, mesh: &Weak<Mesh>) {
        if mesh.weak_count() == 1 {
            let Some(ix) = self.mesh_storage.iter().position(|l| Arc::ptr_eq(l, &mesh.upgrade().unwrap())) else { return; };
            self.mesh_storage.remove(ix);
        }
    }

    fn maybe_remove_material(&mut self, material: &Weak<Material>) {
        if material.weak_count() == 1 {
            let Some(ix) = self.material_storage.iter().position(|l| Arc::ptr_eq(l, &material.upgrade().unwrap())) else { return; };
            self.material_storage.remove(ix);
        }
    }
}
