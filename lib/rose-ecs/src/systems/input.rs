use input::Input;
use rose_platform::events::WindowEvent;

impl InputSystem {
    pub fn on_frame(&mut self) {
        self.input.begin_frame();
    }

    pub fn on_event<'ev>(&mut self, event: WindowEvent<'ev>) -> Option<WindowEvent<'ev>> {
        self.input.apply_event(event)
    }
}

#[derive(Debug, Default)]
pub struct InputSystem {
    pub input: Input,
}
