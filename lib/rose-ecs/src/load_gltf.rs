use std::{path::PathBuf, sync::Arc};

use crossbeam_channel::Sender;
use eyre::Result;
use glam::{Mat4, UVec2, vec2, Vec2, Vec3, Vec4};
use gltf::{
    buffer::Data as BufferData,
    camera::Projection as CamProjection,
    image::{Data as ImageData, Format},
    material::AlphaMode,
    Mesh,
    mesh::util::ReadTexCoords,
    Node, texture::{MagFilter, MinFilter, WrappingMode},
};
use image::{
    buffer::ConvertBuffer, DynamicImage, GrayImage, ImageBuffer, Rgb, RgbaImage, RgbImage,
};
use rayon::prelude::*;
use tracing::Instrument;

use rose_core::transform::Transform;
use rose_renderer::material::Vertex;
use violette::texture::{SampleMode, TextureWrap};

use crate::{
    assets::{Material, MeshAsset},
    prelude::*,
};
use crate::assets::Image;

fn count_children(parent: gltf::Node) -> usize {
    1 + parent.children().map(count_children).sum::<usize>()
}

pub async fn load_gltf_scene(path: impl Into<PathBuf>) -> Result<Scene> {
    let path = path.into();
    tracing::info!("Loading scene from '{}'", path.display());
    let _span = tracing::debug_span!("load_gltf_scene", path=%path.display()).entered();
    let (document, buffers, images) = smol::unblock({
        let path = path.clone();
        move || gltf::import(path)
    })
    .instrument(tracing::debug_span!("load_gltf"))
    .await?;
    let gltf_scene = document
        .default_scene()
        .unwrap_or_else(|| document.scenes().next().unwrap());
    tracing::info!("Entering scene {:?}", gltf_scene.name());
    let mut scene = Scene::new(path.parent().unwrap())?;
    let cache = scene.asset_cache();
    scene.with_world_mut(|world| {
        let num_nodes = gltf_scene.nodes().map(count_children).sum::<usize>();
        let reserved_entities = world.reserve_entities(num_nodes as u32).collect::<Vec<_>>();
        let (tx, rx) = crossbeam_channel::unbounded();
        gltf_scene.nodes().par_bridge().for_each(|node| {
            gltf_load_node(&buffers, &images, cache, &reserved_entities, &tx, &node);
        });

        drop(tx);
        for mut cmd in rx {
            cmd.run_on(world);
        }
    });
    Ok(scene)
}

fn gltf_load_node(
    buffers: &[BufferData],
    images: &[ImageData],
    cache: &'static AssetCache,
    reserved_entities: &[Entity],
    tx: &Sender<CommandBuffer>,
    node: &Node,
) {
    tracing::info!("Entering node {:?}", node.name());
    let mut cmd = CommandBuffer::new();
    let transform = Transform::from_matrix(Mat4::from_cols_array_2d(&node.transform().matrix()));
    let mut entity = EntityBuilder::new();
    entity.add(transform);
    if let Some(name) = node.name() {
        entity.add(name.to_string());
    }

    if let Some(camera) = node.camera() {
        if let CamProjection::Perspective(pers) = camera.projection() {
            entity.add(CameraParams {
                zrange: pers.znear()..pers.zfar().unwrap_or(1e6),
                fovy: pers.yfov(),
            });
        }
    }

    cmd.insert(reserved_entities[node.index()], entity.build());
    let entity = reserved_entities[node.index()];
    if let Some(mesh) = node.mesh() {
        load_node_mesh(buffers, images, cache, mesh)
            .into_par_iter()
            .fold(CommandBuffer::new, |mut cmd, mut builder| {
                cmd.spawn_child(entity, &mut builder);
                cmd
            })
            .for_each(|cmd| tx.send(cmd).unwrap());
    }
    node.children()
        .par_bridge()
        .for_each(|node| gltf_load_node(buffers, images, cache, reserved_entities, tx, &node));
    tx.send(cmd).unwrap();
}

fn load_node_mesh(
    buffers: &[BufferData],
    images: &[ImageData],
    cache: &'static AssetCache,
    mesh: Mesh,
) -> Vec<EntityBuilder> {
    let mesh_name = mesh
        .name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("mesh.{:03}", mesh.index()));
    tracing::info!("Got mesh {:?}", mesh_name);
    mesh.primitives()
        .par_bridge()
        .map(|prim| {
            tracing::info!("Primitive {:?}", prim.index());
            let mut child_entity = EntityBuilder::new();
            child_entity
                .add(Active)
                .add(format!("prim#{:03}", prim.index()))
                .add(Transform::default());
            let reader = prim.reader(|buffer| Some(&buffers[buffer.index()]));
            tracing::info!("\tPositions   : {}", reader.read_positions().is_some());
            tracing::info!("\tNormals     : {}", reader.read_normals().is_some());
            tracing::info!("\tTex coords 0: {}", reader.read_tex_coords(0).is_some());
            let data = reader.read_positions().and_then(|pos| {
                reader.read_normals().and_then(|normals| {
                    reader
                        .read_tex_coords(0)
                        .map(|uv| (pos, normals, coerce_gltf_uv(uv)))
                })
            });
            if let Some((pos, norm, uv)) = data {
                let vertices = pos
                    .map(Vec3::from)
                    .zip(norm.map(Vec3::from).zip(uv))
                    .map(|(pos, (norm, uv))| Vertex::new(pos, norm, uv))
                    .collect::<Vec<_>>();
                let indices: Vec<_> = reader
                    .read_indices()
                    .map(|ix| ix.into_u32().collect())
                    .unwrap_or_else(|| (0..vertices.len() as u32).collect());
                let id = format!("{}.{:03}", mesh_name, prim.index());
                tracing::info!(
                    "Primitive mesh of {} vertices and {} indices",
                    vertices.len(),
                    indices.len()
                );
                let handle = cache.get_or_insert(&id, MeshAsset { indices, vertices });
                child_entity.add(handle);
            }
            let pbr = prim.material().pbr_metallic_roughness();
            let color = pbr.base_color_texture().map(|tex| {
                let texture = &images[tex.texture().source().index()];
                let sampler = tex.texture().sampler();
                let image = image2image(texture);
                Image {
                    image: Arc::new(image),
                    wrap_u: wrap2wrap(sampler.wrap_s()),
                    wrap_v: wrap2wrap(sampler.wrap_t()),
                    sample_min: filter_min2sample(sampler.min_filter()),
                    sample_mag: filter_mag2sample(sampler.mag_filter()),
                }
            });
            let rough_metal = pbr.metallic_roughness_texture().map(|tex| {
                let image = image2image(&images[tex.texture().source().index()]).into_rgb32f();
                let sampler = tex.texture().sampler();
                let data = image
                    .pixels()
                    .flat_map(|px| [px[1], px[2], 0.])
                    .collect::<Vec<_>>();
                let image = DynamicImage::ImageRgb32F(
                    ImageBuffer::from_raw(image.width(), image.height(), data).unwrap(),
                );
                Image {
                    image: Arc::new(image),
                    wrap_u: wrap2wrap(sampler.wrap_s()),
                    wrap_v: wrap2wrap(sampler.wrap_t()),
                    sample_min: filter_min2sample(sampler.min_filter()),
                    sample_mag: filter_mag2sample(sampler.mag_filter()),
                }
            });
            let (normal_amount, normal) = prim
                .material()
                .normal_texture()
                .map(|tex| {
                    let texture = &images[tex.texture().source().index()];
                    let sampler = tex.texture().sampler();
                    let image = image2image(texture);
                    let image = Image {
                        image: Arc::new(image),
                        wrap_u: wrap2wrap(sampler.wrap_s()),
                        wrap_v: wrap2wrap(sampler.wrap_t()),
                        sample_min: filter_min2sample(sampler.min_filter()),
                        sample_mag: filter_mag2sample(sampler.mag_filter()),
                    };
                    (tex.scale(), Some(image))
                })
                .unwrap_or((0., None));
            let emission = prim.material().emissive_texture().map(|tex| {
                let texture = &images[tex.texture().source().index()];
                let sampler = tex.texture().sampler();
                let image = image2image(texture);
                Image {
                    image: Arc::new(image),
                    wrap_u: wrap2wrap(sampler.wrap_s()),
                    wrap_v: wrap2wrap(sampler.wrap_t()),
                    sample_min: filter_min2sample(sampler.min_filter()),
                    sample_mag: filter_mag2sample(sampler.mag_filter()),
                }
            });
            let material = Material {
                transparent: prim.material().alpha_mode() != AlphaMode::Opaque,
                color,
                color_factor: Vec4::from(pbr.base_color_factor()).truncate(),
                normal,
                normal_amount,
                rough_metal,
                rough_metal_factor: vec2(pbr.roughness_factor(), pbr.metallic_factor()),
                emission,
                emission_factor: prim.material().emissive_factor().into(),
            };
            child_entity
                .add(cache.get_or_insert(&format!("prim.{:03}.material", prim.index()), material));
            child_entity
        })
        .collect()
}

fn filter_min2sample(filter: Option<MinFilter>) -> (SampleMode, SampleMode) {
    match filter {
        Some(MinFilter::Linear | MinFilter::LinearMipmapLinear) | None => {
            (SampleMode::Linear, SampleMode::Linear)
        }
        Some(MinFilter::Nearest | MinFilter::NearestMipmapNearest) => {
            (SampleMode::Nearest, SampleMode::Nearest)
        }
        Some(MinFilter::LinearMipmapNearest) => (SampleMode::Linear, SampleMode::Nearest),
        Some(MinFilter::NearestMipmapLinear) => (SampleMode::Nearest, SampleMode::Linear),
    }
}

fn filter_mag2sample(filter: Option<MagFilter>) -> SampleMode {
    match filter {
        Some(MagFilter::Nearest) => SampleMode::Nearest,
        Some(MagFilter::Linear) | None => SampleMode::Linear,
    }
}

fn wrap2wrap(mode: WrappingMode) -> TextureWrap {
    match mode {
        WrappingMode::ClampToEdge => TextureWrap::ClampEdge,
        WrappingMode::MirroredRepeat => TextureWrap::MirroredRepeat,
        WrappingMode::Repeat => TextureWrap::Repeat,
    }
}

fn image2image(texture: &ImageData) -> DynamicImage {
    let image = match texture.format {
        Format::R8 => DynamicImage::ImageLuma8(
            GrayImage::from_raw(texture.width, texture.height, texture.pixels.clone()).unwrap(),
        ),
        Format::R8G8 => DynamicImage::ImageRgb8(
            RgbImage::from_raw(
                texture.width,
                texture.height,
                texture
                    .pixels
                    .windows(2)
                    .flat_map(|v| [v[0], v[1], 0])
                    .collect(),
            )
            .unwrap()
            .convert(),
        ),
        Format::R8G8B8 => DynamicImage::ImageRgb8(
            RgbImage::from_raw(texture.width, texture.height, texture.pixels.clone()).unwrap(),
        ),
        Format::R8G8B8A8 => DynamicImage::ImageRgba8(
            RgbaImage::from_raw(texture.width, texture.height, texture.pixels.clone()).unwrap(),
        ),
        Format::R16 => DynamicImage::ImageLuma16(
            ImageBuffer::from_raw(
                texture.width,
                texture.height,
                bytemuck::cast_slice(&texture.pixels).to_vec(),
            )
            .unwrap(),
        ),
        Format::R16G16 => DynamicImage::ImageRgb32F(
            ImageBuffer::<Rgb<_>, Vec<_>>::from_raw(
                texture.width,
                texture.height,
                bytemuck::cast_slice::<_, u16>(&texture.pixels)
                    .windows(2)
                    .flat_map(|v| [v[0], v[1], 0])
                    .collect(),
            )
            .unwrap()
            .convert(),
        ),
        Format::R16G16B16 => DynamicImage::ImageRgb16(
            ImageBuffer::from_raw(
                texture.width,
                texture.height,
                bytemuck::cast_slice(&texture.pixels).to_vec(),
            )
            .unwrap(),
        ),
        Format::R16G16B16A16 => DynamicImage::ImageRgba16(
            ImageBuffer::from_raw(
                texture.width,
                texture.height,
                bytemuck::cast_slice(&texture.pixels).to_vec(),
            )
            .unwrap(),
        ),
        Format::R32G32B32FLOAT => DynamicImage::ImageRgb32F(
            ImageBuffer::from_raw(
                texture.width,
                texture.height,
                bytemuck::cast_slice(&texture.pixels).to_vec(),
            )
            .unwrap(),
        ),
        Format::R32G32B32A32FLOAT => DynamicImage::ImageRgba32F(
            ImageBuffer::from_raw(
                texture.width,
                texture.height,
                bytemuck::cast_slice(&texture.pixels).to_vec(),
            )
            .unwrap(),
        ),
    };
    image.flipv()
}

fn coerce_gltf_uv(uv: ReadTexCoords) -> impl Iterator<Item = Vec2> {
    let data: Vec<_> = match uv {
        ReadTexCoords::F32(v) => v.map(Vec2::from).collect(),
        ReadTexCoords::U16(u) => u
            .map(|[u, v]| UVec2::from([u as _, v as _]).as_vec2() / u16::MAX as f32)
            .collect(),
        ReadTexCoords::U8(u) => u
            .map(|[u, v]| UVec2::from([u as _, v as _]).as_vec2() / u8::MAX as f32)
            .collect(),
    };
    data.into_iter()
}
