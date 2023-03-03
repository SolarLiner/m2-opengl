use glam::{Quat, Vec2, vec2, Vec3};
use hecs::World;
use input::Input;
use rose_core::transform::Transform;
use rose_platform::{
    events::MouseButton,
    LogicalSize
};
use crate::components::PanOrbitCamera;

#[derive(Debug)]
pub struct PanOrbitSystem {
    pub sensitivity: f32,
    logical_window_size: Vec2,
}

impl PanOrbitSystem {
    pub fn frame_manual(&self, input: &Input, controller: &mut PanOrbitCamera, cam_transform: &mut Transform) {
        let aspect_ratio = self.logical_window_size.x / self.logical_window_size.y;
        let delta = vec2(aspect_ratio, 1.) * input.mouse.delta().truncate() / self.logical_window_size * self.sensitivity;
        let scroll = input.mouse.delta().z;
        self.frame_one(input, delta, scroll, controller, cam_transform);
    }
    
    fn frame_one(&self, input: &Input, delta: Vec2, scroll: f32, controller: &mut PanOrbitCamera, cam_transform: &mut Transform) {
        if input.mouse.state.is_pressed(&MouseButton::Left) {
            cam_transform.rotation *=
                Quat::from_rotation_y(delta.x) * Quat::from_rotation_x(delta.y);
        }
        if input.mouse.state.is_pressed(&MouseButton::Right) {
            let pos = cam_transform.right() * delta.x + cam_transform.down() * delta.y;
            controller.focus += pos;
        }

        controller.radius -= 0.2 * controller.radius * scroll;
        controller.radius = f32::max(0.05, controller.radius);
        cam_transform.position = controller.focus - controller.radius * Vec3::Z;
    }
}

impl PanOrbitSystem {
    pub fn new(size: LogicalSize<f32>) -> Self {
        Self {
            sensitivity: 100.,
            logical_window_size: Vec2::from_array(size.into()),
        }
    }

    pub fn set_window_size(&mut self, size: LogicalSize<f32>) {
        self.logical_window_size = Vec2::from_array(size.into());
    }

    pub fn on_frame(&self, input: &Input, world: &mut World) {
        let aspect_ratio = self.logical_window_size.x / self.logical_window_size.y;
        let delta = vec2(aspect_ratio, 1.) * input.mouse.delta().truncate() / self.logical_window_size * self.sensitivity;
        let scroll = input.mouse.delta().z;
        for (_, (transform, pan_orbit)) in world
            .query::<(&mut Transform, &mut PanOrbitCamera)>()
            .iter()
        {
            self.frame_one(input, delta, scroll, pan_orbit, transform);
        }
    }
}
