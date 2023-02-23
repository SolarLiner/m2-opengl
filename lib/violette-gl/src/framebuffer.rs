use std::{
    fmt::{self, Formatter},
    num::NonZeroU32,
};



use gl::types::GLenum;
use violette_api::base::Resource;
use violette_api::{
    bind::Bind,
    framebuffer::{DrawMode, Framebuffer as ApiFramebuffer},
};

use crate::{api::OpenGLError, context::OpenGLContext, Gl, GlObject, set_ext_label, get_ext_label};

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
            .field(if let Some(id) = &self.id.map(|id| id.get()) {
                id
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
    pub(crate) fn backbuffer(gl: &Gl) -> Self {
        Self {
            gl: gl.clone(),
            id: None,
        }
    }
}

impl GlObject for Framebuffer {
    const GL_NAME: GLenum = gl::FRAMEBUFFER;

    fn gl(&self) -> &Gl {
        &self.gl
    }

    fn id(&self) -> u32 {
        if let Some(id) = self.id {
            id.get()
        } else {
            0
        }
    }
}

impl Resource for Framebuffer {
    fn set_name(&self, name: impl ToString) {
        set_ext_label(self, name)
    }

    fn get_name(&self) -> Option<String> {
        get_ext_label(self)
    }
}

impl ApiFramebuffer for Framebuffer {
    type Gc = OpenGLContext;
    type Err = OpenGLError;

    fn draw_arrays(
        &self,
        mode: DrawMode,
        count: usize,
    ) -> Result<(), Self::Err> {
        unsafe {
            self.gl.DrawArrays(gl_draw_mode(mode), 0, count as _);
        }
        OpenGLError::guard(&self.gl)
    }

    fn draw_elements(
        &self,
        mode: DrawMode,
        count: usize,
    ) -> Result<(), Self::Err> {
        unsafe {
            self.gl.DrawElements(
                gl_draw_mode(mode),
                count as _,
                gl::UNSIGNED_INT,
                std::ptr::null(),
            );
        }
        OpenGLError::guard(&self.gl)
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
                self.gl.BindFramebuffer(gl::FRAMEBUFFER, id.get());
            }
        } else {
            unsafe { self.gl.BindFramebuffer(gl::FRAMEBUFFER, 0) }
        }
    }

    fn unbind(&self) {
        if self.id.is_some() {
            unsafe {
                self.gl.BindFramebuffer(gl::FRAMEBUFFER, 0);
            }
        }
    }
}
