use std::marker::PhantomData;
use std::num::NonZeroU32;
use violette::{self as api, Bind, DrawMode, GraphicsContext};
use crate::api::OpenGLError;
use crate::context::OpenGLContext;

fn gl_draw_mode(mode: DrawMode) -> u32 {
    match mode {
        DrawMode::Points => gl::POINTS,
        DrawMode::Lines => gl::LINES,
        DrawMode::Triangles => gl::TRIANGLES,
        DrawMode::Quads => gl::QUADS,
    }
}

#[derive(Debug)]
pub struct Framebuffer {
    __non_send: PhantomData<*mut ()>,
    id: NonZeroU32,
}

impl api::Framebuffer for Framebuffer {
    type Err = OpenGLError;
    type Gc = OpenGLContext;

    fn draw_arrays(&self, shader: &<Self::Gc as GraphicsContext>::ShaderModule, vao: &<Self::Gc as GraphicsContext>::VertexArray, mode: DrawMode, count: usize) -> Result<(), Self::Err> {
        shader.bind();
        vao.bind();
        unsafe {
            gl::DrawArrays(gl_draw_mode(mode), 0, count as _);
        }
        OpenGLError::guard()
    }

    fn draw_elements(&self, shader: &<Self::Gc as GraphicsContext>::ShaderModule, vao: &<Self::Gc as GraphicsContext>::VertexArray, mode: DrawMode, count: usize) -> Result<(), Self::Err> {
        shader.bind();
        vao.bind();
        unsafe {
            gl::DrawElements(gl_draw_mode(mode), count as _, gl::UNSIGNED_INT, std::ptr::null());
        }
        OpenGLError::guard()
    }
}

impl Bind for Framebuffer {
    type Id = NonZeroU32;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn bind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.id.get());
        }
    }

    fn unbind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.id.get());
        }
    }
}