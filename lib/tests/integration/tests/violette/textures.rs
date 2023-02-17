use rose_platform::PhysicalSize;
use violette::texture::Texture;
use crate::tests::IntegrationTest;

fn test_upload_download(_: PhysicalSize<f32>) {
    let img = image::load_from_memory_with_format(include_bytes!("../../../../../assets/textures/test.png"), image::ImageFormat::Png).unwrap().to_rgb8();
    let texture = Texture::from_image(img).unwrap();
    let actual_img = texture.download_image::<image::Rgb<_>>(0).unwrap();
    actual_img.save("downloaded.png").unwrap();
}

inventory::submit!(IntegrationTest {
    name: "Texture upload/download",
    test_fn: test_upload_download,
});