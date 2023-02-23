use crate::base::Resource;
use crate::bind::Bind;
use crate::context::GraphicsContext;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DrawMode {
    Points,
    Lines,
    Triangles,
    Quads,
}
pub trait Framebuffer: Resource + Bind {
    type Gc: GraphicsContext;
    type Err: Into<<Self::Gc as GraphicsContext>::Err>;
    fn draw_arrays(&self, mode: DrawMode, count: usize) -> Result<(), Self::Err>;
    fn draw_elements(&self, mode: DrawMode, count: usize) -> Result<(), Self::Err>;
}