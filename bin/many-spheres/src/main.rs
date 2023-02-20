use std::{
    f32::consts::TAU,
    sync::Arc,
    time::{Duration, Instant},
};

use camera_controller::OrbitCameraController;
use glam::{vec2, vec3, UVec2, Vec2, Vec3};
use rand::{seq::SliceRandom, Rng};
use rose_core::{
    light::Light,
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

        renderer.add_lights([
            Light::Ambient {
                color: vec3(0.1, 0.2, 0.4),
            },
            Light::Directional {
                color: Vec3::ONE,
                dir: -Vec3::ONE.normalize(),
            },
        ])?;

        let mut camera_controller = OrbitCameraController::default();
        camera_controller.set_orientation(
            renderer.camera_mut(),
            vec2(10f32.to_radians(), 30f32.to_radians()),
        );
        camera_controller.update(Default::default(), renderer.camera_mut());

        let mesh = Arc::new(MeshBuilder::new(Vertex::new).uv_sphere(0.1, 6, 12)?);
        let materials = [
            [0.1, 0.4, 0.8],
            [0.8, 0.1, 0.4],
            [0.8, 0.4, 0.1],
            [0.6, 0.6, 0.6],
        ]
        .map(|color| Material::create(color, None, [0.2, 0.]).unwrap())
        .map(Arc::new);

        let mut rng = rand::thread_rng();
        let spheres = (0..50)
            .map(|_| {
                let orig_pos = (2. * rng.gen::<Vec3>() - 1.) * 3.;
                let phase = rng.gen::<f32>() * TAU;
                let amplitude = rng.gen_range(0.1..=2.);
                let freq = rng.gen_range(0.1..=1.);
                let material = materials.choose(&mut rand::thread_rng()).unwrap();
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

    fn resize(&mut self, size: rose_platform::PhysicalSize<u32>) -> eyre::Result<()> {
        self.renderer.resize(UVec2::from_array(size.into()))
    }

    fn tick(&mut self, dt: Duration) -> eyre::Result<()> {
        let t = self.start.elapsed().as_secs_f32();
        for sphere in &mut self.spheres {
            let offset_y = f32::sin(sphere.phase + sphere.freq * t * TAU) * sphere.amplitude;
            sphere.transform.position = sphere.orig_pos + Vec3::Y * offset_y;
        }
        self.camera_controller
            .update(dt, self.renderer.camera_mut());
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

    fn ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_menu").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.camera_controller.ui_toolbar(ui);
                self.renderer.ui_toolbar(ui);
            });
        });
        self.camera_controller.ui(ctx);
        self.renderer.ui(ctx);
    }
}

fn main() -> eyre::Result<()> {
    rose_platform::run::<ManySpheres>("Many spheres")
}
