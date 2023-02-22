use std::collections::HashSet;

use cgmath::{Vector2, Vector3};
use num_traits::Zero;
pub use winit::event::{MouseButton, VirtualKeyCode as KeyCode};
use winit::event::MouseScrollDelta;

#[derive(Debug, Clone)]
pub struct Input {
    keys: HashSet<KeyCode>,
    mouse: HashSet<MouseButton>,
    mouse_pos: [Vector3<f32>; 2],
}

impl Default for Input {
    fn default() -> Self {
        Self {
            keys: HashSet::new(),
            mouse: HashSet::new(),
            mouse_pos: [Vector3::zero(); 2],
        }
    }
}

impl Input {
    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.keys.contains(&key)
    }

    // Like `key_pressed` for each passed key, but faster.
    pub fn key_chord_pressed(&self, key: impl IntoIterator<Item = KeyCode>) -> bool {
        let chord = HashSet::from_iter(key);
        self.keys.intersection(&chord).all(|_| true)
    }

    pub fn mouse_pressed(&self, mouse: MouseButton) -> bool {
        self.mouse.contains(&mouse)
    }

    pub fn mouse_state(&self) -> Vector3<f32> {
        self.mouse_pos[1]
    }

    pub fn mouse_state_delta(&self) -> Vector3<f32> {
        self.mouse_pos[1] - self.mouse_pos[0]
    }

    pub fn mouse_pos(&self) -> Vector2<f32> {
        self.mouse_state().xy()
    }

    pub fn mouse_delta(&self) -> Vector2<f32> {
        self.mouse_state_delta().xy()
    }

    pub fn mouse_scroll(&self) -> f32 {
        self.mouse_state().z
    }

    pub fn mouse_scroll_delta(&self) -> f32 {
        self.mouse_state_delta().z
    }

    pub fn new_frame(&mut self) {
        self.mouse_pos[0] = self.mouse_pos[1];
    }

    /// Update the Input state from this Winit event, or return it if hasn't been processed.
    pub fn update_from_event<'a>(
        &mut self,
        event: winit::event::WindowEvent<'a>,
    ) -> Option<winit::event::WindowEvent<'a>> {
        use winit::event::ElementState::*;
        use winit::event::WindowEvent::*;
        match event {
            MouseInput {
                button,
                state: Pressed,
                ..
            } => {
                self.mouse.insert(button);
            }
            MouseInput {
                button,
                state: Released,
                ..
            } => {
                self.mouse.remove(&button);
            }
            KeyboardInput {
                input:
                    winit::event::KeyboardInput {
                        state: Pressed,
                        virtual_keycode: Some(vk),
                        ..
                    },
                ..
            } => {
                self.keys.insert(vk);
            }
            KeyboardInput {
                input:
                    winit::event::KeyboardInput {
                        state: Released,
                        virtual_keycode: Some(vk),
                        ..
                    },
                ..
            } => {
                self.keys.remove(&vk);
            }
            MouseWheel {delta, ..} => {
                match delta {
                    MouseScrollDelta::LineDelta(.., down) => {
                        self.mouse_pos[1].z += down;
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        self.mouse_pos[1].z += pos.y as f32;
                    }
                }
            }
            event => return Some(event),
        }
        None
    }
}
