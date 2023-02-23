use std::{
    fmt,
    marker::PhantomData,
    num::NonZeroU32,
    ops,
    cell::Cell,
    ffi::CString,
    fmt::Formatter,
    sync::atomic::{AtomicUsize, Ordering}
};
use crevice::std140::AsStd140;
use gl::types::*;

use violette_api::{
    bind::Bind,
    context::GraphicsContext,
    vao::{VertexArray as ApiVertexArray, VertexLayout},
    value::{ScalarType, ValueType}
};
use violette_api::base::Resource;

use crate::{api::OpenGLError, context::OpenGLContext, get_ext_label, Gl, GlObject, set_ext_label};
use crate::thread_guard::ThreadGuard;

fn gl_scalar_type(typ: ScalarType) -> u32 {
    match typ {
        ScalarType::Bool => gl::BOOL,
        ScalarType::U8 => gl::UNSIGNED_BYTE,
        ScalarType::U16 => gl::UNSIGNED_SHORT,
        ScalarType::U32 => gl::UNSIGNED_INT,
        ScalarType::I8 => gl::BYTE,
        ScalarType::I16 => gl::SHORT,
        ScalarType::I32 => gl::INT,
        ScalarType::F32 => gl::FLOAT,
        ScalarType::F64 => gl::DOUBLE,
        _ => unreachable!("{:?} unsupported in OpenGL", typ),
    }
}

fn gl_value_type(typ: ValueType) -> u32 {
    let scalar = match typ {
        ValueType::Scalar(scalar) => scalar,
        ValueType::Vector(_, scalar) => scalar,
        ValueType::Matrix(_, _, scalar) => scalar,
    };
    gl_scalar_type(scalar)
}

fn gl_num_components(typ: ValueType) -> i32 {
    match typ {
        ValueType::Scalar(_) => 1,
        ValueType::Vector(i, _) => i as _,
        _ => unreachable!("{:?} is not supported in vertex attributes in OpenGL", typ),
    }
}

pub struct VertexArray {
    gl: Gl,
    id: NonZeroU32,
    num_layouts: AtomicUsize,
}

impl fmt::Debug for VertexArray {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VertexArray").field(&self.id.get()).finish()
    }
}

impl Bind for VertexArray {
    type Id = NonZeroU32;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn bind(&self) {
        unsafe {
            self.gl.BindVertexArray(self.id.get());
        }
    }

    fn unbind(&self) {
        unsafe {
            self.gl.BindVertexArray(0);
        }
    }
}

impl GlObject for VertexArray {
    const GL_NAME: GLenum = gl::VERTEX_ARRAY_OBJECT_EXT;

    fn gl(&self) -> &Gl {
        &self.gl
    }

    fn id(&self) -> u32 {
        self.id.get()
    }
}

impl Resource for VertexArray {
    fn set_name(&self, name: impl ToString) {
        set_ext_label(self, name)
    }

    fn get_name(&self) -> Option<String> {
        get_ext_label(self)
    }
}

impl ApiVertexArray for VertexArray {
    type Gc = OpenGLContext;
    type Err = OpenGLError;

    fn set_layout(
        &self,
        stride: usize,
        layout: impl IntoIterator<IntoIter=impl ExactSizeIterator<Item=VertexLayout>>,
    ) -> Result<(), Self::Err> {
        let iter = layout.into_iter();
        self.num_layouts.store(iter.len(), Ordering::SeqCst);
        for (ix, vl) in iter.enumerate() {
            unsafe {
                self.gl.VertexAttribPointer(
                    ix as _,
                    gl_num_components(vl.typ),
                    gl_value_type(vl.typ),
                    gl::FALSE,
                    stride as _,
                    vl.offset as *const _,
                );
            }
            OpenGLError::guard(&self.gl)?;
        }
        Ok(())
    }

    // This takes care of binding and unbinding since this would too unwieldy to let the user do
    fn bind_buffer<T: 'static + Send + Sync + AsStd140>(&self, ix: usize, buffer: &<Self::Gc as GraphicsContext>::Buffer<T>) -> Result<(), Self::Err> {
        self.bind();
        buffer.bind();
        unsafe {
            self.gl.EnableVertexAttribArray(ix as _);
        }
        OpenGLError::guard(&self.gl)?;
        self.unbind();
        buffer.unbind();
        Ok(())
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        let id = self.id.get();
        unsafe {
            self.gl.DeleteVertexArrays(1, &id);
        }
    }
}

impl VertexArray {
    pub(crate) fn new(gl: &Gl) -> Self {
        Self {
            gl: gl.clone(),
            id: NonZeroU32::new(unsafe {
                let mut id = 0;
                gl.GenVertexArrays(1, &mut id);
                id
            })
            .unwrap(),
            num_layouts: AtomicUsize::new(0),
        }
    }
}
