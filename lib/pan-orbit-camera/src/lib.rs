use std::{
    f32::consts::{PI, TAU},
    time::Duration,
};

use glam::{vec2, Mat3, Quat, Vec2, Vec3};
use winit::event::{ElementState, ModifiersState, MouseButton, MouseScrollDelta, WindowEvent};

use rose_core::camera::Camera;

#[derive(Debug, Clone)]
pub struct OrbitCameraController {
    tgt_rotation: Quat,
    sensitivity: f32,
    focus: Vec3,
    radius: f32,
    ui_window_show: bool,
}

impl Default for OrbitCameraController {
    fn default() -> Self {
        Self {
            tgt_rotation: Quat::IDENTITY,
            sensitivity: 1.,
            focus: Vec3::ZERO,
            radius: 3.,
            ui_window_show: false,
        }
    }
}

impl OrbitCameraController {
    pub fn from_camera(camera: &Camera) -> Self {
        let radius = camera.transform.position.length();
        let focus = camera.transform.position + camera.transform.forward() * radius;
        Self {
            tgt_rotation: camera.transform.rotation,
            focus,
            radius,
            ..Default::default()
        }
    }
    pub fn pan(&mut self, camera: &Camera, input: Vec2) {
        let window_size = vec2(camera.projection.width, camera.projection.height);
        let input =
            self.sensitivity * input / window_size * vec2(window_size.x / window_size.y, 1.);
        let left = camera.transform.left() * input.x;
        let up = camera.transform.down() * input.y;
        let translation = (left + up) * self.radius;
        self.focus += translation;
    }

    pub fn orbit(&mut self, camera: &Camera, input: Vec2) {
        let window_size = vec2(camera.projection.width, camera.projection.height);
        let input = input * self.sensitivity;
        let dx = input.x / window_size.x * TAU;
        let dy = input.y / window_size.y * PI;
        let yaw = Quat::from_rotation_y(-dx);
        let pitch = Quat::from_rotation_x(-dy);
        self.tgt_rotation = (yaw * self.tgt_rotation) * pitch;
    }

    pub fn scroll(&mut self, _camera: &Camera, amt: f32) {
        self.radius -= amt * self.radius * 0.05 * self.sensitivity;
        self.radius = self.radius.max(0.05);
        // self.radius = f32::max(0.05, (1. - amt) * self.radius * 0.2 * self.sensitivity);
    }

    pub fn update(&mut self, _dt: Duration, camera: &mut Camera) {
        camera.transform.rotation = self.tgt_rotation;
        camera.transform.position = self.focus + camera.transform.backward() * self.radius;
        camera.transform = camera.transform.looking_at(self.focus);
    }

    pub fn ui_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.toggle_value(&mut self.ui_window_show, "Camera controls");
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        egui::Window::new("Camera controls")
            .open(&mut self.ui_window_show)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let label_focus = ui.label("Position:");
                    vec3(ui, &mut self.focus).labelled_by(label_focus.id);
                    if ui.button("Reset position").clicked() {
                        self.focus *= 0.;
                    }
                });

                ui.horizontal(|ui| {
                    let sensitivity = ui.label("Sensitivity:");
                    ui.add(egui::DragValue::new(&mut self.sensitivity).clamp_range(0f32..=2.))
                        .labelled_by(sensitivity.id);
                });

                ui.horizontal(|ui| {
                    let pos_label = ui.label("Radius:");
                    ui.add(
                        egui::DragValue::new(&mut self.radius)
                            .clamp_range(0f32..=50.)
                            .speed(0.3),
                    )
                    .labelled_by(pos_label.id);
                });
            });
    }

    pub fn set_orientation(&mut self, _camera_mut: &mut Camera, orientation_radians: Vec2) {
        self.tgt_rotation = Quat::from_rotation_y(-orientation_radians.x)
            * Quat::from_rotation_x(-orientation_radians.y);
    }
}

#[derive(Debug, Copy, Clone)]
pub struct OrbitCameraInteractionController {
    dragging: Option<MouseButton>,
    ctrl_pressed: bool,
    last_mouse_pos: Vec2,
}

impl Default for OrbitCameraInteractionController {
    fn default() -> Self {
        Self {
            dragging: None,
            ctrl_pressed: false,
            last_mouse_pos: Vec2::ZERO,
        }
    }
}

impl OrbitCameraInteractionController {
    pub fn dispatch_event(
        &mut self,
        camera_controller: &mut OrbitCameraController,
        camera: &Camera,
        event: WindowEvent,
    ) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let position = position.cast();
                let position = Vec2::new(position.x, position.y);
                match self.dragging {
                    Some(MouseButton::Left) => {
                        camera_controller.orbit(camera, position - self.last_mouse_pos)
                    }
                    Some(MouseButton::Right) => {
                        camera_controller.pan(camera, position - self.last_mouse_pos)
                    }
                    _ => {}
                }
                self.last_mouse_pos = position;
                true
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if state == ElementState::Pressed {
                    self.dragging = match button {
                        MouseButton::Left if self.ctrl_pressed => Some(MouseButton::Right),
                        MouseButton::Right => Some(MouseButton::Right),
                        MouseButton::Left => Some(MouseButton::Left),
                        _ => None,
                    }
                } else {
                    self.dragging.take();
                }
                true
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(_, y) => {
                    camera_controller.scroll(camera, y);
                    true
                }
                MouseScrollDelta::PixelDelta(delta) => {
                    camera_controller.scroll(camera, delta.y as _);
                    true
                }
            },
            WindowEvent::ModifiersChanged(state) => {
                self.ctrl_pressed = state.contains(ModifiersState::CTRL);
                true
            }
            _ => false,
        }
    }
}

fn vec3(ui: &mut egui::Ui, vec3: &mut Vec3) -> egui::Response {
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut vec3.x).prefix("x:"));
        ui.add(egui::DragValue::new(&mut vec3.y).prefix("y:"));
        ui.add(egui::DragValue::new(&mut vec3.z).prefix("z:"));
    })
    .response
}
