use std::{
    any::{Any, TypeId},
    ops,
    sync::{Arc, RwLock},
};

use dashmap::DashMap;
use egui::{Align, Layout, Ui};
use generational_arena::{Arena, Index};
use uuid::Uuid;

use rose_core::material::Material;
use rose_platform::PhysicalSize;
use rose_renderer::Mesh;

pub mod components;

pub struct Named<T> {
    pub name: Option<String>,
    pub value: T,
}

impl<T> From<T> for Named<T> {
    fn from(value: T) -> Self {
        Self { name: None, value }
    }
}

impl<T> ops::Deref for Named<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> ops::DerefMut for Named<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> Named<T> {
    pub fn into_inner(self) -> T {
        self.value
    }
}

pub trait NamedExt: Sized {
    fn named(self, name: impl ToString) -> Named<Self> {
        Named {
            name: Some(name.to_string()),
            value: self,
        }
    }
}

impl<T> NamedExt for T {}

#[allow(unused_variables)]
pub trait Component: Any + Send + Sync {
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn on_create(&mut self, scene: &mut Scene) -> eyre::Result<()> {
        Ok(())
    }

    fn on_before_render(&mut self, scene: &mut Scene) -> eyre::Result<()> {
        Ok(())
    }

    fn on_resize(&mut self, size: PhysicalSize<u32>, scene: &mut Scene) -> eyre::Result<()> {
        Ok(())
    }

    fn on_destroy(&mut self, scene: &mut Scene) {}

    fn ui(&mut self, ui: &mut Ui, scene: &mut Scene);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct MeshRef(Index);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct MaterialRef(Index);

pub struct Scene {
    meshes: Arena<Named<Mesh>>,
    materials: Arena<Named<Material>>,
    entities: Vec<Entity>,
    pub need_relight: bool,
}

impl ops::Index<MaterialRef> for Scene {
    type Output = Named<Material>;

    fn index(&self, index: MaterialRef) -> &Self::Output {
        &self.materials[index.0]
    }
}

impl ops::Index<MeshRef> for Scene {
    type Output = Named<Mesh>;

    fn index(&self, index: MeshRef) -> &Self::Output {
        &self.meshes[index.0]
    }
}

impl Scene {
    pub fn new() -> Self {
        Self {
            meshes: Arena::new(),
            materials: Arena::new(),
            entities: vec![],
            need_relight: false,
        }
    }

    pub fn add_mesh(&mut self, mesh: impl Into<Named<Mesh>>) -> MeshRef {
        let ix = self.meshes.insert(mesh.into());
        MeshRef(ix)
    }

    pub fn add_material(&mut self, material: impl Into<Named<Material>>) -> MaterialRef {
        let ix = self.materials.insert(material.into());
        MaterialRef(ix)
    }

    pub fn remove_mesh(&mut self, id: MeshRef) -> Option<Named<Mesh>> {
        self.meshes.remove(id.0)
    }

    pub fn remove_material(&mut self, id: MaterialRef) -> Option<Named<Material>> {
        self.materials.remove(id.0)
    }

    pub fn create_entity(&mut self) -> &Entity {
        let entity = Entity::new();
        match self.entities.binary_search_by_key(&entity.id, |e| e.id) {
            Ok(_) => unreachable!("UUID collision"),
            Err(ix) => {
                self.entities.insert(ix, entity);
                &self.entities[ix]
            }
        }
    }

    pub fn remove_entity(&mut self, id: Uuid) -> Option<Entity> {
        let ix = self.entities.binary_search_by_key(&id, |e| e.id).ok()?;
        Some(self.entities.remove(ix))
    }

    pub fn entity(&self, id: Uuid) -> Option<&Entity> {
        let ix = self.entities.binary_search_by_key(&id, |e| e.id).ok()?;
        Some(&self.entities[ix])
    }

    pub fn iter(&self) -> impl '_ + Iterator<Item=&Entity> {
        self.entities.iter()
    }
}

pub struct Entity {
    id: Uuid,
    components: DashMap<TypeId, Arc<RwLock<dyn Component>>>,
}

impl Entity {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn add_component<T: Component>(&mut self, comp: impl Into<Arc<T>>) -> &mut Self {
        self.components.insert(TypeId::of::<T>(), comp);
        self
    }

    pub fn has_component<T: Component>(&self) -> bool {
        self.components.contains_key(&TypeId::of::<T>())
    }

    pub fn get_component<T: Component>(&self) -> Option<impl '_ + ops::Deref<Target = T>> {
        self.components.get(&TypeId::of::<T>())
    }

    pub fn get_component_mut<T: Component>(
        &self,
    ) -> Option<impl '_ + ops::Deref<Target = T> + ops::DerefMut> {
        self.components.get_mut(&TypeId::of::<T>())
    }

    pub fn remove_component<T: Component>(&mut self) -> Option<T> {
        self.components.remove(&TypeId::of::<T>())
    }

    pub(crate) fn ui(&mut self, ui: &mut Ui, scene: &mut Scene) {
        ui.with_layout(Layout::top_down(Align::Min), |ui| {
            for comp in self.components.values() {
                ui.scope(|ui| {
                    ui.collapsing(comp.read().unwrap().name(), |ui| {
                        comp.write().unwrap().ui(ui, scene)
                    });
                });
            }
        });
    }

    fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            components: DashMap::new(),
        }
    }
}
