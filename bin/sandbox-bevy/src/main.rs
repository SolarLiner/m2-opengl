use bevy::app::App;
use bevy::DefaultPlugins;
use bevy::prelude::{Commands, GlobalTransform, ResMut};
use glam::Vec3;
use rose_core::camera::Projection;
use rose_core::material::{Material, Vertex};
use rose_core::mesh::MeshBuilder;
use rose_core::transform::Transform;
use violette::texture::Texture;
use crate::plugins::{RoseRenderer, ShellPlugins};

mod plugins;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ShellPlugins)
        .add_startup_system(setup_renderer_camera)
        .add_startup_system(add_objects)
        .run();
}

fn setup_renderer_camera(mut renderer: ResMut<RoseRenderer>) {
    let camera = renderer.camera_mut();
    camera.transform = Transform::translation(Vec3::ONE).looking_at(Vec3::ZERO);
}

fn add_objects(mut commands: Commands) {
    let mesh = MeshBuilder::new(Vertex::new).uv_sphere(1.0, 32, 32).unwrap();
    let material = Material::create(Texture::load_rgb32f("assets/textures/test.png").unwrap(), None, [0.2, 0.]).unwrap();

    let mesh = crate::plugins::Mesh::from(mesh);
    let material = crate::plugins::Material::from(material);
    commands.spawn((GlobalTransform::default(), mesh, material));
}