use std::{
    hash::{Hash, Hasher},
    ops::Mul,
};
use std::f32::consts::PI;

use glam::{EulerRot, Mat4, Quat, Vec3};

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn rotated_deg(self, rot: Vec3) -> Self {
        self.rotated(rot * PI / 180.)
    }
    pub fn rotated(mut self, rot: Vec3) -> Self {
        self.rotation *= Quat::from_euler(EulerRot::ZYX, rot.z, rot.y, rot.x);
        self
    }
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
        self.looking_at_and_up(target, Vec3::Y)
    }

    pub fn looking_at_and_up(self, target: Vec3, up: Vec3) -> Self {
        Self::from_matrix(
            Mat4::from_scale(self.scale) * Mat4::look_at_rh(self.position, target, up),
        )
    }

    pub fn scaled(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation.normalize(), self.position)
        // Mat4::from_translation(self.position) * Mat4::from_quat(self.rotation) * Mat4::from_scale(self.scale)
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

impl Hash for Transform {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for u in self
            .position
            .to_array()
            .into_iter()
            .chain(self.rotation.to_array())
            .chain(self.scale.to_array())
            .map(|f| f.to_bits())
        {
            u.hash(state);
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

/// Wrapper for values with transforms
#[derive(Debug, Clone, Copy, Default)]
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
