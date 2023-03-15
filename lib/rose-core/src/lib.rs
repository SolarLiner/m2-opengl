extern crate glam;

pub mod camera;
pub mod light;
pub mod mesh;
pub mod screen_draw;
pub mod transform;
pub mod utils;

pub mod prelude {
    pub use crate::camera::{Camera, Projection};
    pub use crate::light::{GpuLight, Light, LightBuffer};
    pub use crate::mesh::{CpuMesh, Mesh, MeshBuilder};
    pub use crate::screen_draw::ScreenDraw;
    pub use crate::transform::{Transform, TransformExt, Transformed};
    pub use crate::utils::reload_watcher::*;
    pub use crate::utils::thread_guard::*;
}
