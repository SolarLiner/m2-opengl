use std::collections::HashSet;
use std::hash::Hash;
use std::time::Instant;

use glam::{Vec3, vec3};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent};

#[derive(Debug, Clone)]
pub struct State<T> {
    pressed: HashSet<T>,
    just_pressed: HashSet<T>,
    just_released: HashSet<T>,
}

impl<T> Default for State<T> {
    fn default() -> Self {
        Self {
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            just_released: HashSet::new(),
        }
    }
}

impl<T: Copy + Eq + Hash> State<T> {
    pub fn begin_frame(&mut self) {
        self.just_released.clear();
        self.just_released.clear();
    }

    pub fn is_pressed(&self, value: &T) -> bool {
        self.pressed.contains(value)
    }

    pub fn just_pressed(&self, value: &T) -> bool {
        self.just_pressed.contains(value)
    }

    pub fn just_released(&self, value: &T) -> bool {
        self.just_released.contains(value)
    }

    fn set(&mut self, value: T) {
        self.pressed.insert(value);
        self.just_pressed.insert(value);
    }

    fn clear(&mut self, value: T) {
        self.pressed.remove(&value);
        self.just_pressed.remove(&value);
        self.just_released.insert(value);
    }
}

#[derive(Debug, Default, Clone)]
pub struct KeyboardInput {
    pub state: State<VirtualKeyCode>,
}

impl KeyboardInput {
    pub fn begin_frame(&mut self) {
        self.state.begin_frame();
    }
}

#[derive(Debug, Clone)]
pub struct MouseInput {
    pub state: State<MouseButton>,
    pub pos: Vec3,
    frame_start: Instant,
    prev_pos: Vec3,
}

impl Default for MouseInput {
    fn default() -> Self {
        Self {
            state: State::default(),
            pos: Vec3::ZERO,
            frame_start: Instant::now(),
            prev_pos: Vec3::ZERO,
        }
    }
}

impl MouseInput {
    pub fn delta(&self) -> Vec3 {
        (self.pos - self.prev_pos) * self.frame_start.elapsed().as_secs_f32()
    }

    pub fn begin_frame(&mut self) {
        self.state.begin_frame();
        self.frame_start = Instant::now();
        self.prev_pos = self.pos;
    }
}

#[derive(Debug, Default, Clone)]
pub struct Input {
    pub keyboard: KeyboardInput,
    pub mouse: MouseInput,
}

impl Input {
    pub fn begin_frame(&mut self) {
        self.mouse.begin_frame();
        self.keyboard.begin_frame();
    }

    pub fn apply_event(&mut self, event: WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => self.mouse.state.set(button),
                ElementState::Released => self.mouse.state.clear(button),
            },
            WindowEvent::KeyboardInput {
                input:
                    winit::event::KeyboardInput {
                        state,
                        virtual_keycode: Some(vk),
                        ..
                    },
                ..
            } => match state {
                ElementState::Pressed => self.keyboard.state.set(vk),
                ElementState::Released => self.keyboard.state.clear(vk),
            },
            WindowEvent::MouseWheel {delta, ..} => {
                self.mouse.pos.z += match delta {
                    MouseScrollDelta::LineDelta(_, y) => 10. * y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as _,
                }
            }
            WindowEvent::CursorMoved {position, ..} => {
                self.mouse.pos = vec3(position.x as _, position.y as _, self.mouse.pos.z);
            }
            _ => return false,
        }
        true
    }
}
