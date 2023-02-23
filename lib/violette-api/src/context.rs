use std::sync::Arc;

use bitflags::bitflags;
use bytemuck::Pod;

use crate::{
    buffer::{Buffer, BufferKind},
    framebuffer::Framebuffer,
    math::{Color, Rect},
    shader::ShaderModule,
    vao::VertexArray,
    window::Window
};

bitflags! {
    pub struct ClearBuffers: u8 {
        const COLOR = 1 << 0;
        const DEPTH = 1 << 1;
        const STENCIL = 1 << 2;
    }
}

pub trait GraphicsContext: Send + Sync {
    type Window: Window<Gc=Self>;
    type Err: Into<<Self::Window as Window>::Err>;
    type Buffer<T: 'static + Send + Sync + Pod>: Buffer<T, Gc=Self>;
    type Framebuffer: Framebuffer<Gc=Self>;
    type VertexArray: VertexArray<Gc=Self>;
    type ShaderModule: ShaderModule<Gc=Self>;

    fn backbuffer(&self) -> Arc<Self::Framebuffer>;
    fn clear(&self, mode: ClearBuffers);
    fn set_line_width(&self, width: f32);
    fn set_clear_stencil(&self, stencil: i32);
    fn set_clear_depth(&self, depth: f64);
    fn set_clear_color(&self, color: Color);
    fn set_scissor_test(&self, enabled: bool);
    fn set_depth_test(&self, enabled: bool);
    fn viewport(&self, rect: Rect<f32>);
    fn create_buffer<T: Send + Sync + Pod>(&self, kind: BufferKind) -> Result<Arc<Self::Buffer<T>>, Self::Err>;
    fn create_vertex_array(&self) -> Result<Arc<Self::VertexArray>, Self::Err>;
    fn create_shader_module(&self) -> Result<Arc<Self::ShaderModule>, Self::Err>;
    fn create_framebuffer(&self) -> Result<Arc<Self::Framebuffer>, Self::Err>;
    fn swap_buffers(&self);
}