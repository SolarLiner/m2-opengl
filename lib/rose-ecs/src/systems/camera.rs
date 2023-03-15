use std::f32::consts::{FRAC_PI_2, TAU};

use glam::{vec2, Quat, Vec2, Vec2Swizzles, Vec3};
use hecs::World;

use input::Input;
use rose_core::transform::Transform;
use rose_platform::{events::MouseButton, LogicalSize};

use crate::components::PanOrbitCamera;

#[derive(Debug)]
pub struct PanOrbitSystem {
    pub mouse_sensitivity: f32,
    pub scroll_sensitivity: f32,
    logical_window_size: Vec2,
}

impl PanOrbitSystem {
    pub fn new(size: LogicalSize<f32>) -> Self {
        Self {
            mouse_sensitivity: 3.,
            scroll_sensitivity: 0.1,
            logical_window_size: Vec2::from_array(size.into()),
        }
    }

    pub fn set_window_size(&mut self, size: LogicalSize<f32>) {
        self.logical_window_size = Vec2::from_array(size.into());
    }

    pub fn on_frame(&self, input: &Input, world: &mut World) {
        let aspect_ratio = self.logical_window_size.x / self.logical_window_size.y;
        let delta = vec2(aspect_ratio, 1.) * input.mouse.delta().truncate()
            / self.logical_window_size
            * self.mouse_sensitivity;
        let scroll = input.mouse.delta().z * self.scroll_sensitivity;
        let buttons = (
            input.mouse.state.is_pressed(&MouseButton::Left),
            input.mouse.state.is_pressed(&MouseButton::Right),
        );
        for (_, (transform, pan_orbit)) in world
            .query::<(&mut Transform, &mut PanOrbitCamera)>()
            .iter()
        {
            self.frame_one(delta, scroll, buttons, pan_orbit, transform);
        }
    }

    pub fn with_sensitivity(mut self, sensitivity: f32) -> PanOrbitSystem {
        self.mouse_sensitivity = sensitivity;
        self
    }

    pub fn resize(&mut self, size: LogicalSize<f32>) {
        self.logical_window_size = Vec2::from_array(size.into());
    }

    pub fn frame_manual(
        &self,
        input: &Input,
        controller: &mut PanOrbitCamera,
        cam_transform: &mut Transform,
    ) {
        let aspect_ratio = self.logical_window_size.x / self.logical_window_size.y;
        let delta = vec2(aspect_ratio, 1.) * input.mouse.delta().truncate()
            / self.logical_window_size
            * self.mouse_sensitivity;
        let scroll = input.mouse.delta().z;
        let buttons = (
            input.mouse.state.is_pressed(&MouseButton::Left),
            input.mouse.state.is_pressed(&MouseButton::Right),
        );
        self.frame_one(delta, scroll, buttons, controller, cam_transform);
    }

    pub fn frame_one(
        &self,
        delta: Vec2,
        scroll: f32,
        (left, right): (bool, bool),
        controller: &mut PanOrbitCamera,
        cam_transform: &mut Transform,
    ) {
        if left {
            controller.target_rotation += delta;
        }
        if right {
            let pos = (delta.xy() * vec2(1., -1.)).extend(0.) * controller.radius;
            controller.focus += pos * controller.radius;
        }

        // Clamping
        if controller.target_rotation.x > TAU {
            controller.target_rotation.x -= TAU;
        }
        controller.target_rotation.y = controller.target_rotation.y.clamp(-FRAC_PI_2, FRAC_PI_2);

        controller.radius -= 0.2 * controller.radius * scroll;
        controller.radius = f32::max(0.05, controller.radius);
        // cam_transform.rotation = Quat::from_euler(
        //     EulerRot::XYZ,
        //     controller.target_rotation.y,
        //     controller.target_rotation.x,
        //     0.,
        // );
        cam_transform.rotation = Quat::from_rotation_x(controller.target_rotation.y)
            * Quat::from_rotation_y(controller.target_rotation.x);
        cam_transform.position = controller.focus - controller.radius * Vec3::Z;
        // *cam_transform = cam_transform.looking_at(controller.focus);
    }
}
