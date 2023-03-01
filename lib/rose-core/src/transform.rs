use std::ops::Mul;

use float_ord::FloatOrd;
use glam::{Mat4, Quat, Vec3};

use crate::camera::Camera;

#[derive(Debug, Copy, Clone)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn left(&self) -> Vec3 {
        -self.right()
    }

    pub fn right(&self) -> Vec3 {
        self.rotation.mul_vec3(Vec3::X)
    }

    pub fn up(&self) -> Vec3 {
        self.rotation.mul_vec3(Vec3::Y)
    }

    pub fn down(&self) -> Vec3 {
        -self.up()
    }

    pub fn forward(&self) -> Vec3 {
        self.rotation.mul_vec3(-Vec3::Z)
    }

    pub fn backward(&self) -> Vec3 {
        -self.forward()
    }

    pub fn distance_to_camera(&self, camera: &Camera) -> FloatOrd<f32> {
        let dist = self.position.distance(camera.transform.position);
        FloatOrd(dist)
    }
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

impl Mul<Self> for Transform {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let result_mat = self.matrix() * rhs.matrix();
        Self::from_matrix(result_mat)
    }
}

impl From<Vec3> for Transform {
    fn from(v: Vec3) -> Self {
        Self::translation(v)
    }
}

impl From<Quat> for Transform {
    fn from(v: Quat) -> Self {
        Self::rotation(v)
    }
}

impl From<Mat4> for Transform {
    fn from(mat: Mat4) -> Self {
        Self::from_matrix(mat)
    }
}

impl Transform {
    pub fn translation(pos: Vec3) -> Self {
        Self {
            position: pos,
            ..Default::default()
        }
    }

    pub fn rotation(quat: Quat) -> Self {
        Self {
            rotation: quat,
            ..Default::default()
        }
    }

    pub fn from_matrix(mat: Mat4) -> Self {
        let (scale, rotation, position) = mat.to_scale_rotation_translation();
        Self {
            position,
            rotation,
            scale,
        }
    }

    pub fn looking_at(self, target: Vec3) -> Self {
        Self::from_matrix(
            Mat4::from_scale(self.scale) * Mat4::look_at_rh(self.position, target, Vec3::Y),
        )
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

/// Wrapper for values with transforms
#[derive(Debug, Clone, Copy)]
pub struct Transformed<T> {
    pub value: T,
    pub transform: Transform,
}

impl<T> Transformed<T> {
    pub fn map<U>(self, mapping: impl FnOnce(T) -> U) -> Transformed<U> {
        Transformed {
            value: mapping(self.value),
            transform: self.transform,
        }
    }
}

impl<T> std::ops::Deref for Transformed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> std::ops::DerefMut for Transformed<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

pub trait TransformExt: Sized {
    fn transformed(self, transform: Transform) -> Transformed<Self> {
        Transformed {
            value: self,
            transform,
        }
    }
}

impl<T: Sized> TransformExt for T {}
