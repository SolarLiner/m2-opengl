use std::rc::Rc;
use std::time::Duration;

use eyre::Result;
use glam::{UVec2, Vec2, vec3, Vec3};

use camera_controller::OrbitCameraController;
use rose_core::{
    light::Light,
    mesh::MeshBuilder,
    transform::{Transform, TransformExt},
};
use rose_core::camera::Camera;
use rose_core::utils::thread_guard::ThreadGuard;
use rose_platform::{
    Application,
    events::{ElementState, ModifiersState, MouseButton, MouseScrollDelta, WindowEvent}, PhysicalSize, RenderContext, TickContext, UiContext, WindowBuilder,
};
use rose_renderer::{Mesh, Renderer};
use rose_renderer::material::{MaterialInstance, Vertex};
use violette::texture::Texture;

mod camera_controller;

struct App {
    camera: Camera,
    renderer: ThreadGuard<Renderer>,
    mesh: ThreadGuard<Rc<Mesh>>,
    material: ThreadGuard<Rc<MaterialInstance>>,
    transform: Transform,
    ctrl_pressed: bool,
    dragging: Option<MouseButton>,
    last_mouse_pos: Vec2,
    camera_controller: OrbitCameraController,
}

impl Application for App {
    fn window_features(wb: WindowBuilder) -> WindowBuilder {
        wb.with_inner_size(PhysicalSize::new(1024, 1024))
    }

    #[tracing::instrument(target = "App::new")]
    fn new(size: PhysicalSize<f32>, _scale_factor: f64) -> Result<Self> {
        let base_dir = std::env::current_dir().unwrap();
        let _sizef = Vec2::from_array(size.into());
        let size = UVec2::from_array(size.cast::<u32>().into());
        let mesh = MeshBuilder::new(Vertex::new).uv_sphere(1.0, 32, 64).upload()?.into();
        let mut material = MaterialInstance::create(
            Texture::load_rgb32f("assets/textures/moon_color.png")?,
            Texture::load_rgb32f("assets/textures/moon_normal.png")?,
            None,
        )?;
        material.update_uniforms(|u| {
            u.rough_metal_factor = [0.5, 0.].into();
            u.normal_amount = 0.1;
        })?;
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
        let mut camera = Camera::default();
        let mut camera_controller = OrbitCameraController::default();
        let mut renderer = Renderer::new(size, base_dir)?;
        renderer.add_lights(lights)?;
        camera_controller.update(Duration::default(), &mut camera);

        Ok(Self {
            renderer: ThreadGuard::new(renderer),
            camera,
            camera_controller,
            ctrl_pressed: false,
            dragging: None,
            last_mouse_pos: Vec2::ZERO,
            material: ThreadGuard::new(Rc::new(material)),
            mesh: ThreadGuard::new(Rc::new(mesh)),
            transform: Transform::default(),
        })
    }

    fn resize(&mut self, _size: PhysicalSize<u32>, _scale_factor: f64) -> Result<()> {
        let size = UVec2::from_array(_size.into());
        let sizef = size.as_vec2();
        self.camera.projection.update(sizef);
        self.renderer.resize(size)
    }

    fn interact(&mut self, event: WindowEvent) -> Result<()> {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let position = position.cast();
                let position = Vec2::new(position.x, position.y);
                match self.dragging {
                    Some(MouseButton::Left) => self
                        .camera_controller
                        .orbit(&mut self.camera, position - self.last_mouse_pos),
                    Some(MouseButton::Right) => self
                        .camera_controller
                        .pan(&mut self.camera, position - self.last_mouse_pos),
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
                MouseScrollDelta::LineDelta(_, y) => {
                    self.camera_controller.scroll(&mut self.camera, y)
                }
                MouseScrollDelta::PixelDelta(delta) => self
                    .camera_controller
                    .scroll(&mut self.camera, delta.y as _),
            },
            WindowEvent::ModifiersChanged(state) => {
                self.ctrl_pressed = state.contains(ModifiersState::CTRL)
            }
            _ => {}
        }
        Ok(())
    }
    #[tracing::instrument(target = "App::tick", skip(self))]
    fn tick(&mut self, ctx: TickContext) -> Result<()> {
        self.camera_controller.update(ctx.dt, &mut self.camera);
        Ok(())
    }

    #[tracing::instrument(target = "App::render", skip_all)]
    fn render(&mut self, ctx: RenderContext) -> Result<()> {
        self.renderer.begin_render(&self.camera)?;
        self.renderer.submit_mesh(
            Rc::downgrade(&self.material),
            Rc::downgrade(&self.mesh).transformed(self.transform),
        );
        self.renderer.flush(ctx.dt, Vec3::ZERO)?;
        Ok(())
    }

    fn ui(&mut self, ctx: UiContext) {
        egui::TopBottomPanel::top("top_menu").show(ctx.egui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("Scene", |ui| {
                    let pp_iface = self.renderer.post_process_interface();
                    ui.horizontal(|ui| {
                        let exposure_label = ui.label("Exposure:");
                        ui.add(
                            egui::Slider::new(&mut pp_iface.exposure, 1e-6..=100.)
                                .logarithmic(true)
                                .show_value(true)
                                .custom_formatter(|v, _| format!("{:+1.1} EV", v.log2()))
                                .text("Exposure"),
                        )
                        .labelled_by(exposure_label.id);
                    });

                    ui.horizontal(|ui| {
                        let bloom_size_label = ui.label("Bloom size:");
                        ui.add(
                            egui::Slider::new(&mut pp_iface.bloom.size, 1e-4..=1f32)
                                .logarithmic(true)
                                .show_value(true)
                                .text("Bloom size"),
                        )
                        .labelled_by(bloom_size_label.id);
                    });

                    ui.horizontal(|ui| {
                        let bloom_strength_label = ui.label("Bloom strength:");
                        ui.add(
                            egui::Slider::new(&mut pp_iface.bloom.strength, 1e-4..=1f32)
                                .logarithmic(true)
                                .show_value(true)
                                .text("Bloom strength"),
                        )
                        .labelled_by(bloom_strength_label.id);
                    });
                });
                self.camera_controller.ui_toolbar(ui);
                self.renderer.ui_toolbar(ui);
            });
        });
        self.camera_controller.ui(ctx.egui);
        self.renderer.ui(ctx.egui);

        egui::Window::new("FPS")
            .frame(egui::Frame::none().fill(egui::Color32::from_black_alpha(10)))
            .collapsible(false)
            .title_bar(false)
            .show(ctx.egui, |ui| {
                ui.label(format!("{:>3.1} FPS", ctx.stats.fps_average()));
                egui::plot::Plot::new("fps")
                    .view_aspect(2.)
                    .height(30.)
                    .include_y(0.)
                    .show(ui, |ui| {
                        ui.line(egui::plot::Line::new(
                            ctx.stats
                                .fps_history()
                                .enumerate()
                                .map(|(i, y)| [i as _, y as f64])
                                .collect::<Vec<_>>(),
                        ));
                    });
                ui.label(format!(
                    "50% {:>2.1} | 90% {:>2.1} | 99% {:>2.1}",
                    ctx.stats.percentile(50),
                    ctx.stats.percentile(90),
                    ctx.stats.percentile(99)
                ));
            });
    }
}

fn main() -> Result<()> {
    rose_platform::run::<App>("UV Sphere")
}
