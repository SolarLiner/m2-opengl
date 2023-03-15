use assets_manager::{
    loader::TomlLoader, AnyCache, Asset, BoxedError, Compound, Handle, SharedString,
};
use eyre::WrapErr;
use glam::{vec3, EulerRot, Quat, Vec3};
use hecs::Bundle;
use serde::{Deserialize, Serialize};

use rose_core::transform::Transform;

use crate::assets::{Material, MeshAsset};
use crate::components::Active;

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TransformDesc {
    Direct {
        #[serde(default)]
        translation: Vec3,
        #[serde(default)]
        rotation: Vec3,
        #[serde(default)]
        scale: Vec3,
    },
    LookAt {
        eye: Vec3,
        target: Vec3,
    },
}

impl From<Transform> for TransformDesc {
    fn from(value: Transform) -> Self {
        let (c, b, a) = value.rotation.to_euler(EulerRot::ZYX);
        Self::Direct {
            translation: value.position,
            rotation: vec3(c, b, a),
            scale: value.scale,
        }
    }
}

impl From<TransformDesc> for Transform {
    fn from(val: TransformDesc) -> Self {
        match val {
            TransformDesc::Direct {
                translation,
                rotation,
                scale,
            } => Transform {
                position: translation,
                rotation: Quat::from_euler(EulerRot::YXZ, rotation.y, rotation.x, rotation.z),
                scale,
            },
            TransformDesc::LookAt { eye, target } => Transform::translation(eye).looking_at(target),
        }
    }
}

impl Default for TransformDesc {
    fn default() -> Self {
        Self::Direct {
            translation: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
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
        tracing::debug!(message="Loading object", %id);
        let obj = cache.load::<ObjectDesc>(id)?.cloned();
        let mesh = cache.load(&obj.mesh)?.cloned();
        let material = cache.load(&obj.material)?.cloned();
        Ok(Self { mesh, material })
    }
}

#[derive(Debug, Clone, Bundle)]
pub struct ObjectBundle<M: 'static = Material> {
    pub transform: Transform,
    pub mesh: Handle<'static, MeshAsset>,
    pub material: Handle<'static, M>,
    pub active: Active,
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
            active: Active,
        })
    }
}
