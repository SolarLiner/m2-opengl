use crate::bind::Bind;
use crate::context::GraphicsContext;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DrawMode {
    Points,
    Lines,
    Triangles,
    Quads,
}
pub trait Framebuffer: Bind + Send + Sync {
    type Gc: GraphicsContext;
    type Err: Into<<Self::Gc as GraphicsContext>::Err>;
    fn draw_arrays(&self, shader: &<Self::Gc as GraphicsContext>::ShaderModule, vao: &<Self::Gc as GraphicsContext>::VertexArray, mode: DrawMode, count: usize) -> Result<(), Self::Err>;
    fn draw_elements(&self, shader: &<Self::Gc as GraphicsContext>::ShaderModule, vao: &<Self::Gc as GraphicsContext>::VertexArray, mode: DrawMode, count: usize) -> Result<(), Self::Err>;
}