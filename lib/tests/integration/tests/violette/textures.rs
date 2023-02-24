use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use rose_platform::PhysicalSize;
use violette::texture::Texture;

use crate::tests::IntegrationTest;

fn test_upload_download(_: PhysicalSize<f32>) {
    let img = image::load_from_memory_with_format(
        include_bytes!("../../../../../assets/textures/test.png"),
        image::ImageFormat::Png,
    )
    .unwrap()
    .to_rgb8();
    let texture = Texture::from_image(img.clone()).unwrap();
    let actual_img = texture.download_image::<image::Rgb<_>>(0).unwrap();
    actual_img.save("downloaded.png").unwrap();

    assert_eq!(hash_hex(img.as_raw()), hash_hex(actual_img.as_raw()));
}

fn test_download_mipmap(_: PhysicalSize<f32>) {
    let img = image::load_from_memory_with_format(
        include_bytes!("../../../../../assets/textures/test.png"),
        image::ImageFormat::Png,
    )
    .unwrap()
    .to_rgb8();
    let mut texture = Texture::from_image(img).unwrap();
    texture.generate_mipmaps().unwrap();
    let mipmap = texture.download_image::<image::Rgb<_>>(2).unwrap();
    mipmap.save("downloaded_mipmap.png").unwrap();
}

fn test_download_mipmap_last(_: PhysicalSize<f32>) {
    let img = image::load_from_memory_with_format(
        include_bytes!("../../../../../assets/textures/test.png"),
        image::ImageFormat::Png,
    )
    .unwrap()
    .to_rgb8();
    let mut texture = Texture::from_image(img).unwrap();
    texture.generate_mipmaps().unwrap();
    let dimensions = texture.mipmap_size(texture.num_mipmaps() - 1).unwrap();
    eprintln!("Last mipmap dimensions: {:?}", dimensions);
    let mipmap = texture
        .download_image::<image::Rgb<_>>(texture.num_mipmaps() - 1)
        .unwrap();
    eprintln!("First pixel value: {:?}", mipmap[(0, 0)]);
    assert_eq!(mipmap.dimensions(), (1, 1));
}

inventory::submit!(IntegrationTest {
    name: "Texture upload/download",
    test_fn: test_upload_download,
});

inventory::submit!(IntegrationTest {
    name: "Texture mipmap download",
    test_fn: test_download_mipmap,
});

inventory::submit!(IntegrationTest {
    name: "Texture mipmap (last) download",
    test_fn: test_download_mipmap_last,
});

fn hash_hex(h: &impl Hash) -> String {
    let mut hasher = DefaultHasher::new();
    h.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
