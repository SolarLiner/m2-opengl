use std::path::PathBuf;

use color_eyre::Result;
use glam::{Mat4, UVec2, Vec2, vec3, Vec3, Vec4};
use gltf::{
    buffer::Data as BufferData,
    camera::Projection as CamProjection,
    image::{Data as ImageData, Format},
    Mesh,
    mesh::util::ReadTexCoords,
};
use gltf::image::Data;
use image::{
    buffer::ConvertBuffer, DynamicImage, GrayImage, ImageBuffer, Rgb, RgbaImage, RgbImage,
};
use smol::stream::StreamExt;
use tracing::Instrument;

use rose_core::transform::Transform;
use rose_ecs::{
    assets::{Material, MeshAsset, TextureSlot},
    prelude::*,
};
use rose_ecs::assets::Image;
use rose_renderer::material::Vertex;

pub async fn load_gltf_scenes(path: impl Into<PathBuf>) -> Result<Scene> {
    let path = path.into();
    let _span = tracing::debug_span!("load_sltf_scenes", path=%path.display()).entered();
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
    let default_material = cache.get_or_insert(
        "prim:default_material",
        Material {
            color: TextureSlot::Color(Vec3::splat(0.7)),
            normal: None,
            normal_amount: 1.,
            rough_metal: TextureSlot::Color(vec3(0.3, 0., 0.)),
        },
    );
    scene.with_world_mut(|world| {
        for node in gltf_scene.nodes() {
            tracing::info!("Entering node {:?}", gltf_scene.name());
            let transform =
                Transform::from_matrix(Mat4::from_cols_array_2d(&node.transform().matrix()));
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

            let entity = world.spawn(entity.build());
            if let Some(mesh) = node.mesh() {
                let mut entities =
                    load_node_mesh(cache, mesh, default_material, &buffers[..], &images)?;
                world.spawn_children(entity, &mut entities);
            }
        }
        Ok::<_, eyre::Report>(())
    })?;
    Ok(scene)
}

fn load_node_mesh(
    cache: AnyCache<'static>,
    mesh: Mesh,
    default_material: Handle<'static, Material>,
    buffers: &[BufferData],
    images: &[ImageData],
) -> Result<Vec<EntityBuilder>> {
    let mut entities = vec![];
    let mesh_name = mesh
        .name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("mesh.{:03}", mesh.index()));
    tracing::info!("Got mesh {:?}", mesh_name);
    let primitives = mesh.primitives();
    for prim in primitives {
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
        let color_slot = pbr
            .base_color_texture()
            .map(|tex| {
                let texture = &images[tex.texture().index()];
                gltf_image_into_texture_slot(texture)
            })
            .unwrap_or(TextureSlot::Color(
                Vec4::from(pbr.base_color_factor()).truncate(),
            ));
        let rough_metal = pbr.metallic_roughness_texture().map(|tex| {
            let texture = &images[tex.texture().index()];
            gltf_image_into_texture_slot(texture)
        }).unwrap_or(TextureSlot::Color(vec3(pbr.roughness_factor(), pbr.metallic_factor(), 0.)));
        let normal = prim.material().normal_texture().map(|tex| {
            let texture = &images[tex.texture().index()];
            gltf_image_into_asset(texture)
        });
        let material = Material {
            color: color_slot,
            normal,
            rough_metal,
            normal_amount: 1.,
        };
        child_entity.add(cache.get_or_insert(&format!("prim.{:03}.material", prim.index()), material));
        entities.push(child_entity);
    }
    Ok(entities)
}

fn gltf_image_into_texture_slot(texture: &Data) -> TextureSlot {
    let image = gltf_image_into_asset(texture);
    TextureSlot::Texture(image.into())
}

fn gltf_image_into_asset(texture: &Data) -> Image {
    let image = match texture.format {
        Format::R8 => DynamicImage::ImageLuma8(
            GrayImage::from_raw(texture.width, texture.height, texture.pixels.clone())
                .unwrap(),
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
            RgbImage::from_raw(texture.width, texture.height, texture.pixels.clone())
                .unwrap(),
        ),
        Format::R8G8B8A8 => DynamicImage::ImageRgba8(
            RgbaImage::from_raw(texture.width, texture.height, texture.pixels.clone())
                .unwrap(),
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
    image.into()
}

fn coerce_gltf_uv(uv: ReadTexCoords) -> impl Iterator<Item=Vec2> {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rose_ecs::systems::hierarchy::Parent;

    use super::*;

    #[test]
    fn test_open_glb() {
        let path: PathBuf =
            "/home/solarliner/Documents/Projets/Univ/m2-opengl/assets/gltf/CesiumBalloon.glb"
                .into();
        let mut scene = smol::block_on(load_gltf_scenes(&path)).unwrap();
        let entity_count = scene.with_world(|world, _| {
            let mut count = 0;
            for (entity, _) in world.query::<()>().iter() {
                count += 1;
                let entity = world.entity(entity).unwrap();
                tracing::info!(
                    "Entity {:?}",
                    entity
                        .get::<&String>()
                        .map(|s| (&*s).clone())
                        .unwrap_or_else(|| format!("{:?}", entity.entity()))
                );
                if let Some(parent) = entity.get::<&Parent>() {
                    tracing::info!("\tParent of {:?}", parent.0);
                }
                if let Some(transform) = entity.get::<&Transform>() {
                    tracing::info!("\tLocal transform: {:?}", &*transform);
                }
                if let Some(params) = entity.get::<&CameraParams>() {
                    tracing::info!("\tCamera params: {:?}", &*params);
                }
                if let Some(mesh) = entity.get::<&Handle<MeshAsset>>() {
                    let mesh = mesh.read();
                    tracing::info!(
                        "\tMesh assets: {{ vertices: <len {}>, indices: <len {}> }}",
                        mesh.vertices.len(),
                        mesh.indices.len()
                    );
                }
                if let Some(material) = entity.get::<&Handle<Material>>() {
                    let material = material.read();
                    tracing::info!("\tMaterial assets: {:?}", &*material);
                }
            }
            count
        });
        assert_eq!(0, entity_count);
    }
}
