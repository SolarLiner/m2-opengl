use std::{
    f32::consts::{PI, TAU},
    sync::{Arc, Mutex},
    time::Duration,
};

use camera_controller::OrbitCameraController;
use eyre::{Context, Result};
use glam::{vec2, vec3, Mat3, Quat, UVec2, Vec2, Vec3};

use rose_core::{
    camera::{Camera, Projection},
    gbuffers::GeometryBuffers,
    light::{GpuLight, Light},
    material::{Material, Vertex},
    mesh::MeshBuilder,
    postprocess::Postprocess,
    transform::{Transform, TransformExt},
};
use rose_platform::{
    events::{
        ElementState, KeyboardInput, ModifiersState, MouseButton, MouseScrollDelta, VirtualKeyCode,
        WindowEvent,
    },
    Application, PhysicalSize, WindowBuilder,
};
use rose_renderer::{Mesh, Renderer};
use violette::{
    framebuffer::Framebuffer,
    framebuffer::{ClearBuffer, DepthTestFunction},
    texture::Texture,
    Cull,
};

mod camera_controller;

struct App {
    renderer: Renderer,
    mesh: Arc<Mesh>,
    material: Arc<Material>,
    transform: Transform,
    ctrl_pressed: bool,
    dragging: Option<MouseButton>,
    last_mouse_pos: Vec2,
    camera_controller: camera_controller::OrbitCameraController,
}

impl Application for App {
    fn window_features(wb: WindowBuilder) -> WindowBuilder {
        wb.with_inner_size(PhysicalSize::new(1024, 1024))
    }

    #[tracing::instrument(target = "App::new")]
    fn new(size: PhysicalSize<f32>) -> Result<Self> {
        let sizef = Vec2::from_array(size.into());
        let size = UVec2::from_array(size.cast::<u32>().into());
        let mesh = MeshBuilder::new(Vertex::new).uv_sphere(1.0, 32, 64)?;
        let material = Material::create(
            Texture::load_rgb32f("assets/textures/moon_color.png")?,
            Texture::load_rgb32f("assets/textures/moon_normal.png")?,
            [0.8, 0.0],
        )?
        .with_normal_amount(0.1)?;
        let lights = [
            Light::Directional {
                dir: Vec3::X,
                color: Vec3::ONE * 12.,
            },
            Light::Directional {
                dir: Vec3::Z,
                color: vec3(1., 1.5, 2.),
            },
        ];
        let mut camera_controller = OrbitCameraController::default();
        let mut renderer = Renderer::new(size)?;
        renderer.add_lights(lights)?;
        camera_controller.update(Duration::default(), renderer.camera_mut());

        Ok(Self {
            renderer,
            camera_controller,
            ctrl_pressed: false,
            dragging: None,
            last_mouse_pos: Vec2::ZERO,
            material: Arc::new(material),
            mesh: Arc::new(mesh),
            transform: Transform::default(),
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) -> Result<()> {
        self.renderer.resize(UVec2::from_array(size.into()))
    }

    fn interact(&mut self, event: WindowEvent) -> Result<()> {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let position = position.cast();
                let position = Vec2::new(position.x, position.y);
                match self.dragging {
                    Some(MouseButton::Left) => self
                        .camera_controller
                        .orbit(self.renderer.camera_mut(), position - self.last_mouse_pos),
                    Some(MouseButton::Right) => self
                        .camera_controller
                        .pan(self.renderer.camera_mut(), position - self.last_mouse_pos),
                    _ => {}
                }
                self.last_mouse_pos = position;
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if state == ElementState::Pressed {
                    self.dragging = match button {
                        MouseButton::Right | MouseButton::Left if self.ctrl_pressed => {
                            Some(MouseButton::Right)
                        }
                        MouseButton::Left => Some(MouseButton::Left),
                        _ => None,
                    }
                } else {
                    self.dragging.take();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(_, y) => self.camera_controller.scroll(self.renderer.camera_mut(), y),
                MouseScrollDelta::PixelDelta(delta) => {
                    self.camera_controller.scroll(self.renderer.camera_mut(), delta.y as _)
                }
            },
            WindowEvent::ModifiersChanged(state) => {
                self.ctrl_pressed = state.contains(ModifiersState::CTRL)
            }
            _ => {}
        }
        Ok(())
    }
    #[tracing::instrument(target = "App::tick", skip(self))]
    fn tick(&mut self, dt: Duration) -> Result<()> {
        self.camera_controller
            .update(dt, self.renderer.camera_mut());
        Ok(())
    }

    #[tracing::instrument(target = "App::render", skip_all)]
    fn render(&mut self) -> Result<()> {
        self.renderer.begin_render()?;
        self.renderer
            .submit_mesh(Arc::downgrade(&self.material), Arc::downgrade(&self.mesh).transformed(self.transform));
        self.renderer.flush()?;
        Ok(())
    }

    fn ui(&mut self, ctx: &egui::Context) {
        egui::Window::new("Camera controls").show(ctx, |ui| {
            self.camera_controller.ui(ui);
            let pp_iface = self.renderer.post_process_interface();
            let exposure_label = ui.label("Exposure:");
            ui.add(
                egui::Slider::new(&mut pp_iface.exposure, 1e-6..=10.)
                    .logarithmic(true)
                    .show_value(true)
                    .custom_formatter(|v, _| format!("{:+1.1} EV", v.log2()))
                    .text("Exposure"),
            )
            .labelled_by(exposure_label.id);

            let bloom_size_label = ui.label("Bloom size:");
            ui.add(
                egui::Slider::new(&mut pp_iface.bloom.size, 1e-4..=1f32)
                    .logarithmic(true)
                    .show_value(true)
                    .text("Bloom size"),
            )
            .labelled_by(bloom_size_label.id);

            let bloom_strength_label = ui.label("Bloom strength:");
            ui.add(
                egui::Slider::new(&mut pp_iface.bloom.strength, 1e-4..=1f32)
                    .logarithmic(true)
                    .show_value(true)
                    .text("Bloom strength"),
            )
            .labelled_by(bloom_strength_label.id);
        });
    }
}

fn main() -> Result<()> {
    rose_platform::run::<App>("UV Sphere")
}
