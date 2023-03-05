use std::{borrow::Cow, error::Error, fmt, fmt::Formatter};

use assets_manager::{Asset, BoxedError, Compound, loader::Loader};
use eyre::Result;
use glam::{Quat, vec2, Vec2, vec3, Vec3};

use rose_renderer::material::Vertex;

pub mod obj;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct StringError(pub String);

impl Error for StringError {}

impl fmt::Display for StringError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct DynamicMeshLoader {}

impl Loader<MeshAsset> for DynamicMeshLoader {
    fn load(content: Cow<[u8]>, ext: &str) -> Result<MeshAsset, BoxedError> {
        match ext {
            "obj" => obj::WavefrontLoader::load(content, ext),
            ext => {
                return Err(Box::new(StringError(format!(
                    "Cannot load {:?} as a mesh",
                    ext
                ))))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeshAsset {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

fn quad(_center: Vec3, normal: Vec3) -> [Vertex; 4] {
    let rot = Quat::from_rotation_arc(Vec3::NEG_Z, normal);
    #[rustfmt::skip]
    const QUADS: [Vec2; 4] = [
        vec2(-1., -1.),
        vec2(-1., 1.),
        vec2(1., 1.),
        vec2(1., -1.),
    ];
    QUADS.map(|v| Vertex::new(rot.mul_vec3(v.extend(0.)), normal, v / 2. + 0.5))
}

impl Asset for MeshAsset {
    const EXTENSIONS: &'static [&'static str] = &["obj"];
    type Loader = DynamicMeshLoader;
}

impl MeshAsset {
    pub fn cube() -> Self {
        const FACE_NORMALS: [Vec3; 6] = [
            Vec3::Z,
            Vec3::NEG_Z,
            Vec3::Y,
            Vec3::NEG_Y,
            Vec3::X,
            Vec3::NEG_X,
        ];
        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);

        let mut i = 0;
        for face in FACE_NORMALS {
            vertices.extend_from_slice(&quad(face, face));
            indices.extend_from_slice(&[i, i + 1, i + 2, i, i + 2, i + 3]);
            i += 4;
        }

        Self { vertices, indices }
    }

    pub fn uv_sphere(radius: f32, nlon: usize, nlat: usize) -> Self {
        use std::f32::consts::*;
        let mut vertices = Vec::with_capacity(nlon * nlat + 2);
        let num_triangles = nlon * nlat * 2;
        let mut indices = Vec::with_capacity(num_triangles * 3);

        let lat_step = PI / (nlat - 1) as f32;
        let lon_step = TAU / (nlon - 1) as f32;

        vertices.push(Vertex::new(Vec3::Y * radius, Vec3::Y, vec2(0.5, 1.)));
        for j in 1..nlat {
            let phi = FRAC_PI_2 - j as f32 * lat_step;
            for i in 0..nlon {
                let theta = i as f32 * lon_step;
                let (sphi, cphi) = phi.sin_cos();
                let (sth, cth) = theta.sin_cos();
                let normal = vec3(cphi * cth, sphi, cphi * sth);
                let position = normal * radius;
                let uv = vec2(i as f32 / nlon as f32, 1. - j as f32 / nlat as f32);
                vertices.push(Vertex::new(position, normal, uv));
            }
        }
        vertices.push(Vertex::new(-Vec3::Y * radius, -Vec3::Y, vec2(0.5, 1.)));

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

        Self {
            vertices,
            indices: indices.into_iter().map(|i| i as _).collect(),
        }
    }
}
