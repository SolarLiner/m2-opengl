use float_ord::FloatOrd;
use glam::{vec2, vec3, Vec2, Vec3};
use eyre::{Context, Result};

use violette_low::{
    base::{
        resource::ResourceExt
    },
    buffer::{
        Buffer,
    },
    vertex::{
        DrawMode,
        AsVertexAttributes,
        VertexArray
    }
};
use violette_low::base::resource::Resource;
use violette_low::buffer::{ArrayBuffer, ElementBuffer};
use violette_low::framebuffer::Framebuffer;
use violette_low::program::Program;

use crate::transform::Transform;

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

impl AsVertexAttributes for Vertex {
    type Attr = (Vec3, Vec3, Vec2);
}

#[derive(Debug)]
pub struct Mesh {
    pub transform: Transform,
    vertices: ArrayBuffer<Vertex>,
    array: VertexArray,
    indices: ElementBuffer<u32>,
}

impl Mesh {
    pub fn new(
        vertices: impl IntoIterator<Item = Vertex>,
        indices: impl IntoIterator<Item = u32>,
    ) -> Result<Self> {
        let vertices = vertices.into_iter().collect::<Vec<_>>();
        let vertices = Buffer::with_data(&vertices)?;
        let indices = indices.into_iter().collect::<Vec<_>>();
        let indices = Buffer::with_data(&indices)?;

        let mut vao = VertexArray::new();
        vao.with_vertex_buffer(&vertices)?;
        vao.with_element_buffer(&indices)?;
        Ok(Self {
            transform: Transform::default(),
            vertices,
            array: vao,
            indices,
        })
    }

    pub fn uv_sphere(radius: f32, nlon: usize, nlat: usize) -> Result<Self> {
        use std::f32::consts::*;
        let mut vertices = Vec::with_capacity(nlon * nlat + 2);
        let num_triangles = nlon * nlat * 2;
        let mut indices = Vec::with_capacity(num_triangles * 3);

        let lat_step = PI / (nlat - 1) as f32;
        let lon_step = TAU / (nlon - 1) as f32;

        vertices.push(Vertex {
            position: Vec3::Y,
            uv: vec2(0.5, 1.0),
            normal: Vec3::Y,
        });
        for j in 1..nlat {
            let phi = FRAC_PI_2 - j as f32 * lat_step;
            for i in 0..nlon {
                let theta = i as f32 * lon_step;
                let (sphi, cphi) = phi.sin_cos();
                let (sth, cth) = theta.sin_cos();
                let normal = vec3(cphi * cth, sphi, cphi * sth);
                let position = normal * radius;
                let uv = vec2(i as f32 / nlon as f32, 1. - j as f32 / nlat as f32);
                vertices.push(Vertex {
                    position,
                    normal,
                    uv,
                })
            }
        }
        vertices.push(Vertex {
            position: -Vec3::Y,
            uv: vec2(0.5, 0.0),
            normal: -Vec3::Y,
        });

        // Indices: first row connected to north pole
        for i in 0..nlon {
            indices.extend([0, i + 2, i + 1])
        }

        // Triangles strips
        for lat in 0..nlat - 1 {
            let row_start = lat * nlon + 1;
            for lon in 0..nlon {
                let corner_tl = row_start + lon;
                let corner_tr = corner_tl + 1;
                let corner_bl = corner_tl + nlon;
                let corner_br = corner_bl + 1;
                // First face (top-left)
                indices.extend([corner_tr, corner_bl, corner_tl]);
                // Second face (bottom-right)
                indices.extend([corner_tr, corner_br, corner_bl]);
            }
        }

        // South pole
        let last_idx = vertices.len() - 1;
        let bottom_row = (nlat - 1) * nlon + 1;
        for i in 0..nlon {
            indices.extend([last_idx, bottom_row + i, bottom_row + i + 1]);
        }

        Self::new(vertices, indices.into_iter().map(|i| i as u32))
    }

    pub fn reset_transform(&mut self) {
        self.transform = Transform::default();
    }

    pub fn transformed(mut self, transform: Transform) -> Self {
        self.transform = transform * self.transform;
        self
    }

    pub fn draw(&self, program: &Program, framebuffer: &Framebuffer, wireframe: bool) -> Result<()> {
        self.indices.bind();
        framebuffer
            .draw_elements(program, &self.array, if wireframe {DrawMode::Lines} else {DrawMode::TrianglesList}, ..)
            .context("Cannot draw mesh")?;
        Ok(())
    }

    pub(crate) fn distance_to_camera(&self, camera: &crate::camera::Camera) -> FloatOrd<f32> {
        FloatOrd(self.transform.position.distance(camera.transform.position))
    }
}
