use std::{marker::PhantomData, num::NonZeroU32, ops};
use std::sync::Arc;
use once_cell::sync::Lazy;

use violette_api::{
    bind::Bind,
    context::GraphicsContext,
    framebuffer::{DrawMode, Framebuffer as ApiFramebuffer},
};

use crate::{api::OpenGLError, context::OpenGLContext};
use crate::api::GlErrorKind;
use crate::thread_guard::ThreadGuard;

fn gl_draw_mode(mode: DrawMode) -> u32 {
    match mode {
        DrawMode::Points => gl::POINTS,
        DrawMode::Lines => gl::LINES,
        DrawMode::Triangles => gl::TRIANGLES,
        DrawMode::Quads => gl::QUADS,
    }
}

#[derive(Debug)]
pub struct FramebufferImpl {
    __non_send: PhantomData<*mut ()>,
    id: Option<NonZeroU32>,
}

impl FramebufferImpl {
    pub(crate) const fn backbuffer() -> Self {
        Self {
            __non_send: PhantomData,
            id: None,
        }
    }
}

#[derive(Debug)]
pub struct Framebuffer(pub(crate) ThreadGuard<FramebufferImpl>);

impl ops::Deref for Framebuffer {
    type Target = FramebufferImpl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Framebuffer {
    pub fn new() -> Self {
        let inner = FramebufferImpl {
            __non_send: PhantomData,
            id: unsafe {
                let mut id = 0;
                gl::GenFramebuffers(1, &mut id);
                // Yes this is weird, but the meanings of the two `Option` differ here
                Some(NonZeroU32::new(id as _).unwrap())
            },
        };
        Self(ThreadGuard::new(inner))
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            unsafe {
                let id = id.get();
                gl::DeleteFramebuffers(1, &id);
            }
        }
    }
}

impl ApiFramebuffer for Framebuffer {
    type Gc = OpenGLContext;
    type Err = OpenGLError;

    fn draw_arrays(
        &self,
        shader: &<Self::Gc as GraphicsContext>::ShaderModule,
        vao: &<Self::Gc as GraphicsContext>::VertexArray,
        mode: DrawMode,
        count: usize,
    ) -> Result<(), Self::Err> {
        shader.bind();
        vao.bind();
        unsafe {
            gl::DrawArrays(gl_draw_mode(mode), 0, count as _);
        }
        OpenGLError::guard()
    }

    fn draw_elements(
        &self,
        shader: &<Self::Gc as GraphicsContext>::ShaderModule,
        vao: &<Self::Gc as GraphicsContext>::VertexArray,
        mode: DrawMode,
        count: usize,
    ) -> Result<(), Self::Err> {
        shader.bind();
        vao.bind();
        unsafe {
            gl::DrawElements(
                gl_draw_mode(mode),
                count as _,
                gl::UNSIGNED_INT,
                std::ptr::null(),
            );
        }
        OpenGLError::guard()
    }
}

impl Bind for Framebuffer {
    type Id = u32;

    fn id(&self) -> Self::Id {
        self.id.map(|id| id.get()).unwrap_or(0)
    }

    fn bind(&self) {
        if let Some(id) = self.id {
            unsafe {
                gl::BindFramebuffer(gl::FRAMEBUFFER, id.get());
            }
        } else {
            unsafe { gl::BindFramebuffer(gl::FRAMEBUFFER, 0) }
        }
    }

    fn unbind(&self) {
        if self.id.is_some() {
            unsafe {
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            }
        }
    }
}
