use std::{f32::consts::FRAC_PI_4, ops::Range};
use std::hash::{Hash, Hasher};
use assets_manager::SharedString;

use glam::{vec3, Mat4, Quat, Vec3};
use hecs::Bundle;
use serde::{Deserialize, Serialize};

use rose_core::transform::Transform;

#[derive(Debug)]
pub struct Active;

#[derive(Debug)]
pub struct Inactive;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CameraParams {
    pub fovy: f32,
    pub zrange: Range<f32>,
}

impl Default for CameraParams {
    fn default() -> Self {
        Self {
            fovy: FRAC_PI_4,
            zrange: 1e-3..1e3,
        }
    }
}

#[derive(Debug, Clone, Default, Bundle)]
pub struct CameraBundle {
    pub transform: Transform,
    pub params: CameraParams,
}

#[derive(Debug, Copy, Clone)]
pub struct PanOrbitCamera {
    pub target_rotation: Quat,
    pub radius: f32,
    pub focus: Vec3,
}

impl Default for PanOrbitCamera {
    fn default() -> Self {
        let pos = vec3(3., 2., -3.);
        let radius = pos.length();
        let mat = Mat4::look_to_rh(Vec3::ZERO, -pos.normalize(), Vec3::Y);
        let (_, target_rotation, _) = mat.to_scale_rotation_translation();
        Self {
            target_rotation,
            radius,
            focus: Vec3::ZERO,
        }
    }
}

#[derive(Debug, Default, Bundle)]
pub struct PanOrbitCameraBundle {
    pub transform: Transform,
    pub params: CameraParams,
    pub pan_orbit: PanOrbitCamera,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub enum LightKind {
    Ambient,
    Point,
    Directional,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub struct Light {
    pub kind: LightKind,
    pub color: Vec3,
    pub power: f32,
}

impl Hash for Light {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        for f in self.color.to_array() {
            f.to_bits().hash(state);
        }
        self.power.to_bits().hash(state);
    }
}

impl Default for Light {
    fn default() -> Self {
        Self {
            kind: LightKind::Point,
            color: Vec3::ONE,
            power: 100.,
        }
    }
}

#[derive(Debug, Default, Bundle)]
pub struct LightBundle {
    pub light: Light,
    pub transform: Transform,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SceneId(pub SharedString);