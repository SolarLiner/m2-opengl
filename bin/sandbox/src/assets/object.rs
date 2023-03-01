use assets_manager::{
    loader::TomlLoader, AnyCache, Asset, BoxedError, Compound, Handle, SharedString,
};
use eyre::WrapErr;
use glam::{EulerRot, Quat, Vec3, Vec3Swizzles};
use hecs::Bundle;
use serde::{Deserialize, Serialize};

use rose_core::transform::Transform;

use crate::{assets::material::Material, assets::mesh::MeshAsset};

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct TransformDesc {
    pub translation: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl Into<Transform> for TransformDesc {
    fn into(self) -> Transform {
        Transform {
            position: self.translation,
            rotation: Quat::from_euler(
                EulerRot::YXZ,
                self.rotation.y,
                self.rotation.x,
                self.rotation.z,
            ),
            scale: self.scale,
        }
    }
}

impl Default for TransformDesc {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

impl TransformDesc {
    pub fn right(&self) -> Vec3 {
        let [a, b, c] = self.rotation.zyx().to_array();
        let quat = Quat::from_euler(EulerRot::ZYX, a, b, c);
        quat * Vec3::X
    }

    pub fn down(&self) -> Vec3 {
        let [a,b,c] = self.rotation.zyx().to_array();
        let quat = Quat::from_euler(EulerRot::ZYX, a,b,c);
        quat * Vec3::NEG_Y
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ObjectDesc {
    pub mesh: SharedString,
    pub material: SharedString,
}

impl Asset for ObjectDesc {
    const EXTENSION: &'static str = "toml";
    type Loader = TomlLoader;
}

#[derive(Debug, Clone)]
pub struct Object {
    pub mesh: MeshAsset,
    pub material: Material,
}

impl Compound for Object {
    fn load(cache: AnyCache, id: &SharedString) -> eyre::Result<Self, BoxedError> {
        let obj = cache.load::<ObjectDesc>(id)?.cloned();
        let mesh = cache.load(&obj.mesh)?.cloned();
        let material = cache.load(&obj.material)?.cloned();
        Ok(Self { mesh, material })
    }
}

#[derive(Debug, Clone, Bundle)]
pub struct ObjectBundle {
    pub transform: Transform,
    pub mesh: Handle<'static, MeshAsset>,
    pub material: Handle<'static, Material>,
}

impl ObjectBundle {
    pub fn from_asset_cache(
        cache: AnyCache<'static>,
        transform: Transform,
        id: &str,
    ) -> eyre::Result<Self> {
        let desc = cache
            .load::<ObjectDesc>(id)
            .with_context(|| format!("Loading asset {:?}", id))?
            .cloned();
        Ok(Self {
            transform,
            mesh: cache.load(&desc.mesh)?,
            material: cache.load(&desc.material)?,
        })
    }
}
