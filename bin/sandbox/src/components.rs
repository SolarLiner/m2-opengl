use std::{f32::consts::FRAC_PI_4, ops::Range};

use glam::{vec3, Mat4, Quat, Vec3};
use hecs::Bundle;

use rose_core::transform::Transform;

#[derive(Debug, Clone)]
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

#[derive(Debug)]
pub struct Active;

#[derive(Debug)]
pub struct Inactive;

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
