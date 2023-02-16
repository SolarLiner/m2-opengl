use std::ops::Range;

use glam::{Mat4, Vec2};

use crate::transform::Transform;

#[derive(Debug, Clone)]
pub struct Projection {
    pub fovy: f32,
    pub width: f32,
    pub height: f32,
    pub zrange: Range<f32>,
}

impl Default for Projection {
    fn default() -> Self {
        Self {
            fovy: 45f32.to_radians(),
            zrange: 0.001..1000.0,
            width: 1.,
            height: 1.,
        }
    }
}

impl Projection {
    pub fn update(&mut self, size: Vec2) {
        self.width = size.x;
        self.height = size.y;
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::perspective_rh_gl(
            self.fovy,
            self.width / self.height,
            self.zrange.start,
            self.zrange.end,
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct Camera {
    pub transform: Transform,
    pub projection: Projection,
}
