use std::{
    fmt::{
        self,
        Formatter
    },
    marker::PhantomData,
    num::NonZeroU32,
    ops,
    sync::Arc
};

use once_cell::sync::Lazy;

use gl::types::GLenum;
use violette_api::base::Resource;
use violette_api::{
    bind::Bind,
    context::GraphicsContext,
    framebuffer::{DrawMode, Framebuffer as ApiFramebuffer},
};

use crate::{
    api::GlErrorKind, api::OpenGLError, context::OpenGLContext, thread_guard::ThreadGuard, Gl,
    GlObject,
};

fn gl_draw_mode(mode: DrawMode) -> u32 {
    match mode {
        DrawMode::Points => gl::POINTS,
        DrawMode::Lines => gl::LINES,
        DrawMode::Triangles => gl::TRIANGLES,
        _ => unreachable!("OpenGL cannot draw {:?}", mode),
    }
}

pub struct Framebuffer {
    gl: Gl,
    id: Option<NonZeroU32>,
}

impl fmt::Debug for Framebuffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Framebuffer")
            .field(if let Some(id) = self.id {
                &id.get()
            } else {
                &0
            })
            .finish()
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            unsafe {
                let id = id.get();
                self.gl.DeleteFramebuffers(1, &id);
            }
        }
    }
}

impl Framebuffer {
    pub fn new(gl: &Gl) -> Self {
        Self {
            gl: gl.clone(),
            id: unsafe {
                let mut id = 0;
                gl.GenFramebuffers(1, &mut id);
                // Yes this is weird, but the meanings of the two `Option` differ here
                Some(NonZeroU32::new(id as _).unwrap())
            },
        }
    }
}

impl Framebuffer {
    pub(crate) const fn backbuffer(gl: &Gl) -> Self {
        Self {
            gl: gl.clone(),
            id: None,
        }
    }
}

impl Resource for Framebuffer {
    fn set_name(&self, name: impl ToString) {}

    fn get_name(&self) -> Option<String> {
        None
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
            self.gl.DrawArrays(gl_draw_mode(mode), 0, count as _);
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
            self.gl.DrawElements(
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
