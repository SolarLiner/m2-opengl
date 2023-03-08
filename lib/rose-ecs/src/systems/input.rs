use input::Input;
use rose_platform::events::WindowEvent;

impl InputSystem {
    pub fn on_frame(&mut self) {
        self.input.begin_frame();
    }

    pub fn on_event(&mut self, event: WindowEvent) -> bool {
        self.input.apply_event(event)
    }
}

#[derive(Debug, Default)]
pub struct InputSystem {
    pub input: Input,
}
