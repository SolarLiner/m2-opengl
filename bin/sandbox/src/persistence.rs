use std::path::PathBuf;

use glam::{Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::scene::{Entity, Scene};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneFile {
    #[serde(flatten)]
    objects: Vec<Object>,
}

impl SceneFile {
    pub fn into_scene(self) -> Scene {
        let mut scene = Scene::new();
        for obj in self.objects {}
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    name: Option<String>,
    #[serde(default)]
    transform: Transform,
    r#type: ObjectType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Transform {
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl From<rose_core::transform::Transform> for Transform {
    fn from(value: rose_core::transform::Transform) -> Self {
        Self {
            position: value.position,
            scale: value.scale,
            rotation: value.rotation,
        }
    }
}

impl Into<rose_core::transform::Transform> for Transform {
    fn into(self) -> rose_core::transform::Transform {
        rose_core::transform::Transform {
            position: self.position,
            scale: self.scale,
            rotation: self.rotation,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ObjectType {
    Plane { plane: Vec2 },
    Cube { cube: Vec3 },
    Sphere { sphere: f32 },
    Object { object: PathBuf },
    Light { light: Light },
}

impl ObjectType {
    fn create_entity(&self, scene: &mut Scene) -> Entity {
        match self {
            ObjectType::Plane { plane, .. } => {}
            ObjectType::Cube { .. } => {}
            ObjectType::Sphere { .. } => {}
            ObjectType::Object { .. } => {}
            ObjectType::Light { .. } => {}
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Light {
    Ambient { ambient: Vec3 },
    Directional { directional: Vec3 },
    Point { point: Vec3 },
}

impl From<rose_core::light::Light> for Light {
    fn from(value: rose_core::light::Light) -> Self {
        match value {
            Light::Point { color, .. } => Self::Point { point: color },
            Light::Directional { color, .. } => Self::Directional { directional: color },
            Light::Ambient { color, .. } => Self::Ambient { ambient: color },
        }
    }
}

impl Into<rose_core::light::Light> for Light {
    fn into(self) -> rose_core::light::Light {
        match self {
            Self::Ambient { ambient } => rose_core::light::Light::Ambient { color: ambient },
            Self::Directional { directional } => rose_core::light::Light::Directional {
                dir: Vec3::ZERO,
                color: directional,
            },
            Self::Point { point } => rose_core::light::Light::Point {
                color: point,
                position: Vec3::ZERO,
            },
        }
    }
}
