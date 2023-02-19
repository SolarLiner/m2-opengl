use std::{
    collections::HashMap, fs::File, io::BufReader, ops::Deref, path::Path, path::PathBuf, sync::Arc,
};

use eyre::{Context, Result};
use glam::{vec2, vec3, Vec2, Vec3};
use obj::raw::object::Polygon;
use obj::raw::{
    material::{MtlColor, MtlTextureMap},
    RawObj,
};
use once_cell::sync::Lazy;
use smol::stream::StreamExt;
use tracing::Instrument;

use rose_core::{
    transform::Transform,
    material::{
        TextureSlot,
        Material,
        Vertex
    },
    mesh::Mesh,
    transform::TransformExt
};
use violette::texture::Texture;

use crate::scene::Scene;

static WHITE_MATERIAL: Lazy<Arc<Material>> =
    Lazy::new(|| Arc::new(Material::create([1.; 3], None, [0.3; 2]).unwrap()));

pub trait ObjectData {
    fn insert_into_scene(&self, scene: &mut Scene) -> Result<Vec<u64>>;
}

pub trait MeshLoader<D: ObjectData> {
    type Meshes: IntoIterator<Item = D>;

    fn meshes(&self) -> Self::Meshes;
}

pub struct WavefrontLoader {
    filepath: PathBuf,
    raw_obj: RawObj,
    materials: HashMap<String, obj::raw::material::Material>,
    images: HashMap<String, image::DynamicImage>,
}

impl WavefrontLoader {
    pub async fn load(path: impl Into<PathBuf>) -> Result<Self> {
        let filepath = path.into();
        let path = filepath.clone();
        let raw_obj: RawObj = smol::unblock(move || {
            Ok::<_, eyre::Report>(
                obj::raw::parse_obj(BufReader::new(
                    File::open(&path).context("Cannot open mesh file")?,
                ))
                .context("Cannot parse OBJ")?,
            )
        })
        .await?;
        let materials = smol::stream::iter(raw_obj.material_libraries.iter())
            .then(|fpath| {
                let fpath = filepath.parent().unwrap().join(fpath);
                let span = tracing::info_span!("obj::raw::parse_mtl", path=%fpath.display());
                smol::unblock(move || {
                    Ok::<_, eyre::Report>(
                        obj::raw::parse_mtl(BufReader::new(
                            File::open(fpath).context("Cannot open material library")?,
                        ))
                        .context("Cannot parse material library")?,
                    )
                })
                .instrument(span)
            })
            .try_fold(HashMap::new(), |mut acc, val| {
                acc.extend(val.materials);
                Ok(acc)
            })
            .await?;
        let images = smol::stream::iter(materials.values().flat_map(|mat| {
            let mut files = vec![];
            if let Some(map) = &mat.diffuse_map {
                files.push(map.file.to_string());
            }
            if let Some(map) = &mat.bump_map {
                files.push(map.file.to_string());
            }
            if let Some(map) = &mat.specular_map {
                files.push(map.file.to_string());
            }
            files
        }))
        .then(|path| smol::unblock(|| image::open(path.clone()).map(|img| (path, img))))
        .filter_map(|res| match res {
            Ok(img) => Some(img),
            Err(err) => {
                tracing::error!("Couldn't open image: {}", err);
                None
            }
        })
        .fold(HashMap::new(), |mut map, (key, value)| {
            map.insert(key, value);
            map
        })
        .await;
        Ok(Self {
            filepath,
            raw_obj,
            materials,
            images,
        })
    }

    pub fn load_sync(path: impl Into<PathBuf>) -> Result<Self> {
        smol::block_on(Self::load(path))
    }

    async fn convert_mat2(
        &self,
        color: Option<&MtlColor>,
        texture: Option<&MtlTextureMap>,
        name: &str,
    ) -> Result<TextureSlot<2>> {
        Ok(if let Some(tex) = texture {
            self.convert_mat_texture2(tex)
                .await
                .map(TextureSlot::Texture)?
        } else {
            TextureSlot::Color(
                color
                    .and_then(|col| self.convert_mat_color(col, name))
                    .map(|[r, g, _]| [r, g])
                    .unwrap_or([0.2, 0.]),
            )
        })
    }

    async fn convert_mat_texture2(&self, tex: &MtlTextureMap) -> Result<Texture<[f32; 2]>> {
        let file = self.filepath.parent().unwrap().join(&tex.file);
        let image = smol::unblock(move || image::open(file))
            .await
            .context("Cannot open texture")?;
        let width = image.width();
        let data = image
            .into_rgb32f()
            .into_raw()
            .chunks_exact(3)
            .flat_map(|slice| [slice[0], slice[1]])
            .collect::<Vec<_>>();
        Ok(Texture::from_2d_pixels(width.try_into()?, &data)?)
    }

    async fn convert_mat3(
        &self,
        color: Option<&MtlColor>,
        texture: Option<&MtlTextureMap>,
        name: &str,
    ) -> Result<TextureSlot<3>> {
        Ok(if let Some(tex) = texture {
            self.convert_mat_texture3(tex)
                .await
                .map(TextureSlot::Texture)?
        } else {
            TextureSlot::Color(
                color
                    .and_then(|col| self.convert_mat_color(col, name))
                    .unwrap_or([1., 0., 1.]),
            )
        })
    }

    async fn convert_mat_texture3(&self, tex: &MtlTextureMap) -> Result<Texture<[f32; 3]>> {
        let file = self.filepath.parent().unwrap().join(&tex.file);
        tracing::info!("Loading texture {}", file.display());
        let image = smol::unblock(move || image::open(file))
            .await
            .context("Cannot open texture")?;
        Ok(Texture::from_image(image.into_rgb32f())?)
    }

    fn convert_mat_color(&self, color: &MtlColor, mat_name: &str) -> Option<[f32; 3]> {
        match color {
            &MtlColor::Rgb(r, g, b) => Some([r, g, b]),
            col => {
                tracing::warn!(message=format!("Unsupported color {:?}", col), material=mat_name, obj=?self.raw_obj.name);
                None
            }
        }
    }
}

impl ObjectData for WavefrontLoader {
    fn insert_into_scene(&self, scene: &mut Scene) -> Result<Vec<u64>> {
        let scene_materials: HashMap<_, _> = smol::block_on(
            smol::stream::iter(self.materials.iter())
                .then(|(name, mat)| async move {
                    tracing::info!("Loading material {}", name);
                    let albedo = self
                        .convert_mat3(mat.diffuse.as_ref(), mat.diffuse_map.as_ref(), name)
                        .await?;
                    let normal = if let Some(map) = &mat.bump_map {
                        let texture = self.convert_mat_texture3(map).await?;
                        Some(texture)
                    } else {
                        None
                    };
                    let rough_metal = if let Some(tex) = &mat.specular_map {
                        self.convert_mat_texture2(tex)
                            .await
                            .map(TextureSlot::Texture)?
                    } else if let Some(specular) = &mat.specular {
                        let col = self
                            .convert_mat_color(specular, name)
                            .map(|[r, _, _]| [r, 0.])
                            .unwrap_or([0.2, 0.]);
                        TextureSlot::Color(col)
                    } else {
                        TextureSlot::Color([0.2, 0.])
                    };
                    Ok::<_, eyre::Report>((name, Material::create(albedo, normal, rough_metal)?))
                })
                .fold(HashMap::new(), |mut map, res| {
                    let (name, material) = match res {
                        Err(err) => {
                            tracing::error!("Cannot instanciate material for mesh: {}", err);
                            return map;
                        }
                        Ok(ok) => ok,
                    };
                    map.insert(name.to_string(), scene.add_material(material));
                    map
                }),
        );
        Ok(self
            .raw_obj
            .meshes
            .iter()
            .map(|(name, group)| {
                tracing::info!("Loading mesh {}", name);
                let vertices = group
                    .polygons
                    .iter()
                    .copied()
                    .flat_map(|range| &self.raw_obj.polygons[range.start..range.end])
                    .map(|poly| match poly {
                        Polygon::P(pos) => pos
                            .iter()
                            .copied()
                            .map(|ix| {
                                let (x, y, z, _) = self.raw_obj.positions[ix];
                                Vertex::new(vec3(x, y, z), Vec3::ZERO, Vec2::ZERO)
                            })
                            .collect::<Vec<_>>(),
                        Polygon::PT(pt) => pt
                            .iter()
                            .copied()
                            .map(|(ixp, ixt)| {
                                let (x, y, z, _) = self.raw_obj.positions[ixp];
                                let (u, v, _) = self.raw_obj.tex_coords[ixt];
                                Vertex::new(vec3(x, y, z), Vec3::ZERO, vec2(u, v))
                            })
                            .collect(),
                        Polygon::PN(pn) => pn
                            .iter()
                            .copied()
                            .map(|(ixp, ixn)| {
                                let (x, y, z, _) = self.raw_obj.positions[ixp];
                                let (nx, ny, nz) = self.raw_obj.normals[ixn];
                                Vertex::new(vec3(x, y, z), vec3(nx, ny, nz), Vec2::ZERO)
                            })
                            .collect(),
                        Polygon::PTN(ptn) => ptn
                            .iter()
                            .copied()
                            .map(|(ixp, ixt, ixn)| {
                                let (x, y, z, _) = self.raw_obj.positions[ixp];
                                let (nx, ny, nz) = self.raw_obj.normals[ixn];
                                let (u, v, _) = self.raw_obj.tex_coords[ixt];
                                Vertex::new(vec3(x, y, z), vec3(nx, ny, nz), vec2(u, v))
                            })
                            .collect(),
                    })
                    .flatten()
                    .collect::<Vec<_>>();
                let indices = Vec::from_iter(0..vertices.len() as u32);
                let mesh = Mesh::new(vertices, indices)?;
                let mesh = scene.add_mesh(mesh);
                Ok::<_, eyre::Report>((name, mesh))
            })
            .filter_map(|res| match res {
                Ok(s) => Some(s),
                Err(err) => {
                    tracing::error!("Failed to instanciate mesh: {}", err);
                    None
                }
            })
            // Collect here to only have one closuring capturing `scene` running at the same time
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(name, mesh)| {
                let material = scene_materials
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| scene.add_material(WHITE_MATERIAL.deref().clone()));
                scene
                    .instance_object(material, mesh.transformed(Transform::default()))
                    .named(name)
                    .id()
            })
            .collect())
    }
}

impl<T: ObjectData> ObjectData for Box<T> {
    #[inline(always)]
    fn insert_into_scene(&self, scene: &mut Scene) -> Result<Vec<u64>> {
        T::insert_into_scene(&*self, scene)
    }
}

pub trait LoadMeshExt: Sized {
    fn load_object(&mut self, loader: &impl ObjectData) -> Result<Vec<u64>>;
}

impl LoadMeshExt for Scene {
    fn load_object(&mut self, loader: &impl ObjectData) -> Result<Vec<u64>> {
        loader.insert_into_scene(self)
    }
}

#[tracing::instrument(skip_all, fields(path = %path.as_ref().display()))]
pub fn load_mesh_dynamic(
    path: impl AsRef<Path>,
) -> Result<Box<dyn 'static + Sync + Send + ObjectData>> {
    let path = path.as_ref();
    tracing::info!("Loading mesh file");
    let ext = path.extension().map(|s| s.to_string_lossy().to_string());
    Ok(match ext.as_deref() {
        Some("obj") => {
            Box::new(WavefrontLoader::load_sync(path).context("Cannot load Wavefront OBJ")?)
        }
        Some(other) => eyre::bail!("Unknown extension {:?}", other),
        None => eyre::bail!("Cannot determine file format (no extension in path)"),
    })
}
