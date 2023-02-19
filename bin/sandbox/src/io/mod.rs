use std::{
    ops::Deref, path::Path, sync::Arc,
};

use eyre::{Context, Result};
use once_cell::sync::Lazy;
use self::obj::WavefrontLoader;

use rose_core::{
    material::Material,
    transform::TransformExt
};
use crate::scene::Scene;

mod obj;

static WHITE_MATERIAL: Lazy<Arc<Material>> =
    Lazy::new(|| Arc::new(Material::create([1.; 3], None, [0.3; 2]).unwrap()));

pub trait ObjectData {
    fn insert_into_scene(&self, scene: &mut Scene) -> Result<Vec<u64>>;
}

pub trait MeshLoader<D: ObjectData> {
    type Meshes: IntoIterator<Item = D>;

    fn meshes(&self) -> Self::Meshes;
}

impl<T: ObjectData> ObjectData for Box<T> {
    #[inline(always)]
    fn insert_into_scene(&self, scene: &mut Scene) -> Result<Vec<u64>> {
        T::insert_into_scene(&*self, scene)
    }
}

pub trait LoadMeshExt: Sized {
    fn load_object(&mut self, loader: &impl ObjectData) -> Result<Vec<u64>>;
}

impl LoadMeshExt for Scene {
    fn load_object(&mut self, loader: &impl ObjectData) -> Result<Vec<u64>> {
        loader.insert_into_scene(self)
    }
}

#[tracing::instrument(skip_all, fields(path = %path.as_ref().display()))]
pub fn load_mesh_dynamic(
    path: impl AsRef<Path>,
) -> Result<Box<dyn 'static + Sync + Send + ObjectData>> {
    let path = path.as_ref();
    tracing::info!("Loading mesh file");
    let ext = path.extension().map(|s| s.to_string_lossy().to_string());
    Ok(match ext.as_deref() {
        Some("obj") => {
            Box::new(WavefrontLoader::load_sync(path).context("Cannot load Wavefront OBJ")?)
        }
        Some(other) => eyre::bail!("Unknown extension {:?}", other),
        None => eyre::bail!("Cannot determine file format (no extension in path)"),
    })
}
