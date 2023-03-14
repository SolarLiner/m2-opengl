pub use rose_core as core;
pub use rose_ecs as ecs;
pub use input;
pub use rose_platform as platform;
pub use rose_renderer as renderer;
#[cfg(feature = "ui")]
pub use rose_ui as ui;

pub mod prelude {
    pub use input::*;
    pub use rose_core::prelude::*;
    pub use rose_ecs::prelude::*;
    pub use rose_renderer::prelude::*;
    pub use rose_platform::prelude::*;
    pub use eyre::{Result, Context};
    pub use glam::*;
    pub use tracing::{self, trace, debug, info, warn, error};
}