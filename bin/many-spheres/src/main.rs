use std::{f32::consts::TAU, sync::Arc, time::{Instant, Duration}};

use camera_controller::OrbitCameraController;
use glam::{vec2, Vec2, Vec3};
use rose_core::{
    material::{Material, Vertex},
    mesh::MeshBuilder,
    transform::{Transform, TransformExt},
};
use rose_platform::Application;
use rose_renderer::{Mesh, Renderer};
use violette::framebuffer::Framebuffer;

mod camera_controller;

struct Sphere {
    phase: f32,
    amplitude: f32,
    freq: f32,
    orig_pos: Vec3,
    transform: Transform,
    mesh: Arc<Mesh>,
    material: Arc<Material>,
}

struct ManySpheres {
    renderer: Renderer,
    camera_controller: OrbitCameraController,
    spheres: Vec<Sphere>,
    start: Instant,
}

impl Application for ManySpheres {
    fn new(size: rose_platform::PhysicalSize<f32>) -> eyre::Result<Self> {
        let sizef = Vec2::from_array(size.into());
        let size = sizef.as_uvec2();
        let mut renderer = Renderer::new(size)?;
        let mut camera_controller = OrbitCameraController::default();
        camera_controller.set_orientation(
            renderer.camera_mut(),
            vec2(10f32.to_radians(), 30f32.to_radians()),
        );
        camera_controller.update(Default::default(), renderer.camera_mut());

        let mesh = Arc::new(MeshBuilder::new(Vertex::new).uv_sphere(0.1, 12, 24)?);
        let material = Arc::new(Material::create([0.1, 0.4, 0.8], None, [0.2, 0.])?);

        let spheres = (0..10)
            .map(|_| {
                let orig_pos = rand::random::<Vec3>() * 3.;
                let phase = rand::random::<f32>() * TAU;
                let amplitude = 1. + rand::random::<f32>();
                let freq = 0.1 + rand::random::<f32>() * 0.9;
                Sphere {
                    mesh: mesh.clone(),
                    material: material.clone(),
                    orig_pos,
                    transform: Transform {
                        position: orig_pos,
                        ..Default::default()
                    },
                    phase,
                    amplitude,
                    freq,
                }
            })
            .collect();
        Framebuffer::backbuffer().clear_color([0.1, 0.1, 0.4, 1.])?;
        Ok(Self {
            renderer,
            camera_controller,
            spheres,
            start: Instant::now(),
        })
    }

    fn tick(&mut self, _dt: Duration) -> eyre::Result<()> {
        let t = self.start.elapsed().as_secs_f32();
        for sphere in &mut self.spheres {
            let offset_y = f32::sin(sphere.phase + sphere.freq * t * TAU) * sphere.amplitude;
            sphere.transform.position = sphere.orig_pos + Vec3::Y * offset_y;
        }
        Ok(())
    }

    fn render(&mut self) -> eyre::Result<()> {
        self.renderer.begin_render()?;
        for sphere in &self.spheres {
            self.renderer.submit_mesh(
                Arc::downgrade(&sphere.material),
                Arc::downgrade(&sphere.mesh).transformed(sphere.transform),
            );
        }
        self.renderer.flush()
    }
}

fn main() -> eyre::Result<()> {
    rose_platform::run::<ManySpheres>("Many spheres")
}
