use std::f32::consts::FRAC_PI_2;
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct CameraParams {
    pub fovy: f32,
    pub zrange: Range<f32>,
}

impl Default for CameraParams {
    fn default() -> Self {
        Self {
            fovy: FRAC_PI_2,
            zrange: 1e-3..1e3,
        }
    }
}

#[derive(Debug)]
pub struct Active;

#[derive(Debug)]
pub struct Inactive;
