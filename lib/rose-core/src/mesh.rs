use std::marker::PhantomData;

use bytemuck::Pod;
use float_ord::FloatOrd;
use glam::{vec2, vec3, Vec2, Vec3};
use eyre::{Context, Result};

use violette::{
    buffer::{
        Buffer,
    },
    vertex::{
        DrawMode,
        VertexAttributes,
        VertexArray
    }
};

use violette::{buffer::{ArrayBuffer, ElementBuffer}, framebuffer::Framebuffer, program::Program};

use crate::transform::Transform;

#[derive(Debug)]
pub struct Mesh<Vertex> {
    array: VertexArray,
    vertices: ArrayBuffer<Vertex>,
    indices: ElementBuffer<u32>,
}

impl<Vertex: Pod> Mesh<Vertex> where Vertex: VertexAttributes {
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
            vertices,
            array: vao,
            indices,
        })
    }

    pub fn empty() -> Result<Self> {
        let vertices = Buffer::new();
        let indices = Buffer::new();
        let mut vao = VertexArray::new();
        vao.with_vertex_buffer(&vertices)?;
        vao.with_element_buffer(&indices)?;
        Ok(Self {vertices, array: vao, indices })
    }

    pub fn vertices(&mut self) -> &mut ArrayBuffer<Vertex> {
        &mut self.vertices
    }

    pub fn indices(&mut self) -> &mut ElementBuffer<u32> {
        &mut self.indices
    }

    pub fn draw(&self, program: &Program, framebuffer: &Framebuffer, wireframe: bool) -> Result<()> {
        framebuffer
            .draw_elements(program, &self.array, if wireframe {DrawMode::Lines} else {DrawMode::Triangles}, 0..self.indices.len() as i32)
            .context("Cannot draw mesh")?;
        Ok(())
    }
}

pub struct MeshBuilder<Vertex, Ctor> {
    ctor: Ctor,
    __phantom: PhantomData<Vertex>
}

impl<Vtx, Ctor> MeshBuilder<Vtx, Ctor> {
    pub fn new(ctor: Ctor) -> Self { Self { ctor, __phantom: PhantomData } }
}

impl<Vertex: Pod, Ctor: Fn(Vec3, Vec3, Vec2) -> Vertex> MeshBuilder<Vertex, Ctor> where Vertex: VertexAttributes {
    pub fn uv_sphere(&self, radius: f32, nlon: usize, nlat: usize) -> Result<Mesh<Vertex>> {
        use std::f32::consts::*;
        let mut vertices = Vec::with_capacity(nlon * nlat + 2);
        let num_triangles = nlon * nlat * 2;
        let mut indices = Vec::with_capacity(num_triangles * 3);

        let lat_step = PI / (nlat - 1) as f32;
        let lon_step = TAU / (nlon - 1) as f32;

        vertices.push((self.ctor)(Vec3::Y, Vec3::Y, vec2(0.5, 1.)));
        for j in 1..nlat {
            let phi = FRAC_PI_2 - j as f32 * lat_step;
            for i in 0..nlon {
                let theta = i as f32 * lon_step;
                let (sphi, cphi) = phi.sin_cos();
                let (sth, cth) = theta.sin_cos();
                let normal = vec3(cphi * cth, sphi, cphi * sth);
                let position = normal * radius;
                let uv = vec2(i as f32 / nlon as f32, 1. - j as f32 / nlat as f32);
                vertices.push((self.ctor)(position, normal, uv));
            }
        }
        vertices.push((self.ctor)(-Vec3::Y, -Vec3::Y, vec2(0.5, 0.0)));

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

        Mesh::new(vertices, indices.into_iter().map(|i| i as u32))
    }
}