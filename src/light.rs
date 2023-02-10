use anyhow::Context;
use crevice::std140;
use crevice::std140::AsStd140;
use glam::Vec3;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use violette_low::buffer::{BoundBuffer, Buffer, BufferKind};

#[derive(Debug, Copy, Clone, FromPrimitive)]
#[repr(u32)]
pub enum LightType {
    Point = 0,
    Directional = 1,
    Ambient = 2,
}

#[derive(Debug, Copy, Clone)]
pub enum Light {
    Point { color: Vec3, position: Vec3 },
    Directional { color: Vec3, dir: Vec3 },
    Ambient { color: Vec3 },
}

impl Light {
    fn pos_dir(&self) -> Vec3 {
        match self {
            &Self::Point { position, .. } => position,
            &Self::Directional { dir, .. } => dir,
            Self::Ambient { .. } => Vec3::ZERO,
        }
    }

    pub fn kind(&self) -> LightType {
        match self {
            Self::Point { .. } => LightType::Point,
            Self::Directional { .. } => LightType::Directional,
            Self::Ambient { .. } => LightType::Ambient,
        }
    }

    pub fn color(&self) -> Vec3 {
        match self {
            &Self::Directional { color, .. }
            | &Self::Point { color, .. }
            | &Self::Ambient { color } => color,
        }
    }

    pub fn color_mut(&mut self) -> &mut Vec3 {
        match self {
            Self::Directional { color, .. }
            | Self::Point { color, .. }
            | Self::Ambient { color } => color,
        }
    }
}

impl From<GpuLight> for Light {
    fn from(light: GpuLight) -> Self {
        let kind = LightType::from_u32(light.kind).unwrap();
        match kind {
            LightType::Point => Self::Point {
                position: from_std140vec3(light.pos_dir),
                color: from_std140vec3(light.color),
            },
            LightType::Directional => Self::Directional {
                dir: from_std140vec3(light.pos_dir),
                color: from_std140vec3(light.color),
            },
            LightType::Ambient => Self::Ambient {
                color: from_std140vec3(light.color),
            },
        }
    }
}

#[derive(Debug, Clone, AsStd140)]
#[repr(align(64))]
pub struct GpuLight {
    kind: u32,
    pos_dir: std140::Vec3,
    color: std140::Vec3,
}

impl From<Light> for GpuLight {
    fn from(l: Light) -> Self {
        Self {
            kind: l.kind() as _,
            pos_dir: to_std140vec3(l.pos_dir()),
            color: to_std140vec3(l.color()),
        }
    }
}

impl GpuLight {
    pub fn create_buffer(lights: impl IntoIterator<Item = Light>) -> anyhow::Result<LightBuffer> {
        let data = lights
            .into_iter()
            .map(Self::from)
            .map(|v| v.as_std140())
            .collect::<Vec<_>>();
        Buffer::with_data(BufferKind::Uniform, &data).context("Cannot create light buffer")
    }
}

pub type LightBuffer = Buffer<<GpuLight as AsStd140>::Output>;
pub type BoundLightBuffer<'a> = BoundBuffer<'a, <GpuLight as AsStd140>::Output>;

fn from_std140vec3(v: std140::Vec3) -> Vec3 {
    Vec3::from([v.x, v.y, v.z])
}

fn to_std140vec3(v: glam::Vec3) -> std140::Vec3 {
    let [x, y, z] = v.to_array();
    std140::Vec3 { x, y, z }
}
