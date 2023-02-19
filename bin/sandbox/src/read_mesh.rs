use std::fs::File;
use std::io::BufReader;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use glam::Vec3;
use once_cell::sync::Lazy;

use rose_core::material::{Material, Vertex};
use rose_core::mesh::Mesh;
use rose_core::transform::TransformExt;

use crate::scene::{Instance, Scene};

static WHITE_MATERIAL: Lazy<Arc<Material>> =
    Lazy::new(|| Arc::new(Material::create([1.; 3], None, [0.3; 2]).unwrap()));

pub trait ObjectData {
    fn insert_into_scene<'scene>(
        &self,
        scene: &'scene mut Scene,
    ) -> eyre::Result<&'scene mut Instance>;
}

pub trait MeshLoader<D: ObjectData> {
    type Meshes: IntoIterator<Item = D>;

    fn meshes(&self) -> Self::Meshes;
}

impl ObjectData for obj::Obj<obj::TexturedVertex, u32> {
    fn insert_into_scene<'scene>(
        &self,
        scene: &'scene mut Scene,
    ) -> eyre::Result<&'scene mut Instance> {
        let mesh = Mesh::new(
            self.vertices.iter().map(|v| {
                Vertex::new(
                    v.position.into(),
                    v.normal.into(),
                    Vec3::from(v.texture).truncate(),
                )
            }),
            self.indices.iter().copied(),
        )?;
        let mesh = scene.add_mesh(mesh);
        let material = scene.add_material(WHITE_MATERIAL.deref().clone());
        let instance = scene.instance_object(material, mesh.transformed(Default::default()));
        Ok(if let Some(name) = &self.name {
            instance.named(name)
        } else {
            instance
        })
    }
}

impl<T: ObjectData> ObjectData for Box<T> {
    #[inline(always)]
    fn insert_into_scene<'scene>(
        &self,
        scene: &'scene mut Scene,
    ) -> eyre::Result<&'scene mut Instance> {
        T::insert_into_scene(&*self, scene)
    }
}

pub trait LoadMeshExt: Sized {
    fn load_object(&mut self, loader: &impl ObjectData) -> eyre::Result<&mut Instance>;
}

impl LoadMeshExt for Scene {
    fn load_object(&mut self, loader: &impl ObjectData) -> eyre::Result<&mut Instance> {
        loader.insert_into_scene(self)
    }
}

pub fn load_mesh_dynamic(path: impl AsRef<Path>) -> eyre::Result<Box<dyn 'static + Sync + Send + ObjectData>> {
    let path = path.as_ref();
    let ext = path.extension().map(|s| s.to_string_lossy().to_string());
    Ok(match ext.as_deref() {
        Some("obj") => Box::new(obj::load_obj(BufReader::new(File::open(path)?))?),
        Some(other) => eyre::bail!("Unknown extension {:?}", other),
        None => eyre::bail!("Cannot determine file format (no extension in path)"),
    })
}
