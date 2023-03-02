use glam::{Quat, Vec2, vec2, Vec3};
use hecs::World;
use input::Input;
use rose_core::transform::Transform;
use rose_platform::events::MouseButton;
use rose_platform::LogicalSize;
use crate::components::PanOrbitCamera;

#[derive(Debug)]
pub struct PanOrbitSystem {
    sensitivity: f32,
    logical_window_size: Vec2,
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

    pub fn on_frame(&mut self, input: &Input, world: &mut World) {
        let aspect_ratio = self.logical_window_size.x / self.logical_window_size.y;
        let delta = vec2(aspect_ratio, 1.) * input.mouse.delta().truncate() / self.logical_window_size * self.sensitivity;
        let scroll = input.mouse.delta().z;
        for (_, (transform, pan_orbit)) in world
            .query::<(&mut Transform, &mut PanOrbitCamera)>()
            .iter()
        {
            if input.mouse.state.is_pressed(&MouseButton::Left) {
                transform.rotation *=
                    Quat::from_rotation_y(delta.x) * Quat::from_rotation_x(delta.y);
            }
            if input.mouse.state.is_pressed(&MouseButton::Right) {
                let pos = transform.right() * delta.x + transform.down() * delta.y;
                pan_orbit.focus += pos;
            }

            pan_orbit.radius -= 0.2 * pan_orbit.radius * scroll;
            pan_orbit.radius = f32::max(0.05, pan_orbit.radius);
            transform.position = pan_orbit.focus - pan_orbit.radius * Vec3::Z;
        }
    }
}
