use std::{f32::consts::TAU, sync::Arc, time::Instant};
use std::rc::Rc;

use camera_controller::OrbitCameraController;
use glam::{vec2, vec3, UVec2, Vec2, Vec3};
use rand::{seq::SliceRandom, Rng};
use rose_core::{
    light::Light,
    material::{Material, Vertex},
    mesh::MeshBuilder,
    transform::{Transform, TransformExt},
};
use rose_core::camera::Camera;
use rose_core::utils::thread_guard::ThreadGuard;
use rose_platform::{Application, RenderContext, TickContext, UiContext};
use rose_renderer::{Mesh, Renderer};
use violette::framebuffer::Framebuffer;

mod camera_controller;

struct Sphere {
    phase: f32,
    amplitude: f32,
    freq: f32,
    orig_pos: Vec3,
    transform: Transform,
    mesh: ThreadGuard<Rc<Mesh>>,
    material: ThreadGuard<Rc<Material>>,
}

struct ManySpheres {
    camera: Camera,
    renderer: ThreadGuard<Renderer>,
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

        let mut camera = Camera::default();
        camera.projection.width = sizef.x;
        camera.projection.height = sizef.y;
        let mut camera_controller = OrbitCameraController::default();
        camera_controller.set_orientation(
            &mut camera,
            vec2(10f32.to_radians(), 30f32.to_radians()),
        );
        camera_controller.update(Default::default(), &mut camera);

        let mesh = Rc::new(MeshBuilder::new(Vertex::new).uv_sphere(0.1, 6, 12)?);
        let materials = [
            [0.1, 0.4, 0.8],
            [0.8, 0.1, 0.4],
            [0.8, 0.4, 0.1],
            [0.6, 0.6, 0.6],
        ]
        .map(|color| Material::create(color, None, [0.2, 0.]).unwrap())
        .map(Rc::new);

        let mut rng = rand::thread_rng();
        let spheres = (0..50)
            .map(|_| {
                let orig_pos = (2. * rng.gen::<Vec3>() - 1.) * 3.;
                let phase = rng.gen::<f32>() * TAU;
                let amplitude = rng.gen_range(0.1..=2.);
                let freq = rng.gen_range(0.1..=1.);
                let material = materials.choose(&mut rand::thread_rng()).unwrap();
                Sphere {
                    mesh: ThreadGuard::new(mesh.clone()),
                    material: ThreadGuard::new(material.clone()),
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
        Framebuffer::clear_color([0.1, 0.1, 0.4, 1.]);
        Ok(Self {
            renderer: ThreadGuard::new(renderer),
            camera,
            camera_controller,
            spheres,
            start: Instant::now(),
        })
    }

    fn resize(&mut self, size: rose_platform::PhysicalSize<u32>) -> eyre::Result<()> {
        self.renderer.resize(UVec2::from_array(size.into()))
    }

    fn tick(&mut self, ctx: TickContext) -> eyre::Result<()> {
        let t = self.start.elapsed().as_secs_f32();
        for sphere in &mut self.spheres {
            let offset_y = f32::sin(sphere.phase + sphere.freq * t * TAU) * sphere.amplitude;
            sphere.transform.position = sphere.orig_pos + Vec3::Y * offset_y;
        }
        self.camera_controller
            .update(ctx.dt, &mut self.camera);
        Ok(())
    }

    fn render(&mut self, ctx: RenderContext) -> eyre::Result<()> {
        self.renderer.begin_render(Vec3::ZERO.extend(1.))?;
        for sphere in &self.spheres {
            self.renderer.submit_mesh(
                Rc::downgrade(&sphere.material),
                Rc::downgrade(&sphere.mesh).transformed(sphere.transform),
            );
        }
        self.renderer.flush(&self.camera, ctx.dt)
    }

    fn ui(&mut self, ctx: UiContext) {
        egui::TopBottomPanel::top("top_menu").show(ctx.egui, |ui| {
            ui.horizontal(|ui| {
                self.camera_controller.ui_toolbar(ui);
                self.renderer.ui_toolbar(ui);
            });
        });
        self.camera_controller.ui(ctx.egui);
        self.renderer.ui(ctx.egui);
    }
}

fn main() -> eyre::Result<()> {
    rose_platform::run::<ManySpheres>("Many spheres")
}
