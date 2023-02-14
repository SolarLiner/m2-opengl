use std::ops::Mul;

use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Copy, Clone)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn right(&self) -> Vec3 {
        self.rotation.mul_vec3(Vec3::X)
    }
    
    pub fn up(&self) -> Vec3 {
        self.rotation.mul_vec3(Vec3::Y)
    }

    pub fn forward(&self) -> Vec3 {
        self.rotation.mul_vec3(-Vec3::Z)
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
        Mat4::from_rotation_translation(self.rotation, self.position)
    }
}
