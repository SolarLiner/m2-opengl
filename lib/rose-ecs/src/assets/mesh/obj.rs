use std::{borrow::Cow, io::Cursor};

use assets_manager::{BoxedError, loader::Loader};
use glam::Vec3;

use rose_renderer::material::Vertex;

use super::super::mesh::MeshAsset;

pub struct WavefrontLoader {}

impl Loader<MeshAsset> for WavefrontLoader {
    fn load(content: Cow<[u8]>, _ext: &str) -> Result<MeshAsset, BoxedError> {
        let obj = obj::load_obj::<obj::TexturedVertex, _, u32>(Cursor::new(content))?;
        Ok(MeshAsset {
            vertices: obj
                .vertices
                .into_iter()
                .map(|v| {
                    Vertex::new(
                        v.position.into(),
                        v.normal.into(),
                        Vec3::from_array(v.texture).truncate(),
                    )
                })
                .collect(),
            indices: obj.indices,
        })
    }
}

#[cfg(never)]
impl Loader<MtlAsset> for WavefrontLoader {
    fn load(content: Cow<[u8]>, _ext: &str) -> Result<MtlAsset, BoxedError> {
        let mtl = obj::raw::parse_mtl(Cursor::new(content))?;
        Ok(MtlAsset {
            materials: mtl
                .materials
                .into_iter()
                .map(|(k, v)| (k.into(), MtlMaterialAsset::from(v)))
                .collect(),
        })
    }
}

#[cfg(never)]
impl Loader<ObjMesh> for WavefrontLoader {
    fn load(content: Cow<[u8]>, _ext: &str) -> Result<ObjMesh, BoxedError> {
        let obj = obj::raw::parse_obj(Cursor::new(content))?;
        let material_libraries = obj
            .material_libraries
            .into_iter()
            .map(|s| (s.clone().into(), s.into()))
            .collect();
        let meshes = obj
            .groups
            .into_iter()
            .map(|(name, group)| {
                let indices = group
                    .points
                    .into_iter()
                    .flat_map(|r| r.start..r.end)
                    .map(|ix| ix as u32)
                    .collect();
                let vertex = group
                    .polygons
                    .into_iter()
                    .flat_map(|r| r.start..r.end)
                    .map(|pt| {
                        let (x, y, z, _) = obj.positions[pt];
                        let pos = Vec3::new(x, y, z);
                        let (x, y, z) = obj.normals[pt];
                        let normal = Vec3::new(x, y, z);
                        let (u, v, _) = obj.tex_coords[pt];
                        let uv = Vec2::new(u, v);
                        Vertex::new(pos, normal, uv)
                    })
                    .collect::<Vec<_>>();
                (name.into(), MeshAsset { vertices: vertex, indices })
            })
            .collect::<_>();
        Ok(ObjMesh {
            meshes,
            material_libraries,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg(never)]
pub struct MtlMaterialAsset {
    pub color: Option<PathBuf>,
    pub color_factor: Vec3,
    pub normal: Option<PathBuf>,
    pub normal_amount: f32,
    pub roughness: Option<PathBuf>,
    pub roughness_amount: f32,
    pub metal: Option<PathBuf>,
    pub metal_amount: f32,
}

#[cfg(never)]
impl From<obj::raw::material::Material> for MtlMaterialAsset {
    fn from(value: obj::raw::material::Material) -> Self {
        let color = if let Some(map) = value.diffuse_map {
            TextureSlotDesc::Texture(map.file.into())
        } else if let Some(MtlColor::Rgb(r, g, b)) = value.diffuse {
            TextureSlotDesc::Color(vec3(r, g, b))
        } else {
            TextureSlotDesc::Color(Vec3::splat(0.5))
        };
        let normal = value.bump_map.map(|m| m.file.into());
        let roughness = if let Some(map) = value.specular_map {
            TextureSlotDesc::Texture(map.file.into())
        } else if let Some(color) = value.specular {
            let val = match color {
                MtlColor::Rgb(r, ..) => r,
                MtlColor::Xyz(x, ..) => x,
                _ => 0.,
            };
            TextureSlotDesc::Color(Vec3::splat(val))
        } else {
            TextureSlotDesc::Color(Vec3::ZERO)
        };
        let metal = TextureSlotDesc::Color(Vec3::ZERO);
        Self {
            color,
            normal,
            roughness,
            metal,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg(never)]
pub struct MtlAsset {
    materials: HashMap<SharedString, MtlMaterialAsset>,
}

#[cfg(never)]
impl Asset for MtlAsset {
    type Loader = WavefrontLoader;
}

#[derive(Debug, Clone)]
#[cfg(never)]
pub struct ObjMesh {
    material_libraries: HashMap<SharedString, PathBuf>,
    meshes: HashMap<SharedString, MeshAsset>,
}

#[cfg(never)]
impl Asset for ObjMesh {
    type Loader = WavefrontLoader;
}
