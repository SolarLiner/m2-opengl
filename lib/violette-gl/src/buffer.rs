use std::{
    cell::Cell,
    collections::Bound,
    marker::PhantomData,
    num::{NonZeroU32, NonZeroUsize},
    ops,
    ops::{Range, RangeBounds},
};

use crevice::std140::{AsStd140, Std140};
use once_cell::sync::Lazy;

use violette_api::{
    bind::Bind,
    buffer::BufferKind,
    buffer::BufferUsage,
    buffer::{Buffer as ApiBuffer, ReadBuffer, WriteBuffer},
};

use crate::api::OpenGLError;
use crate::context::OpenGLContext;
use crate::thread_guard::ThreadGuard;

fn gl_target(kind: BufferKind) -> u32 {
    match kind {
        BufferKind::UniformBlock => gl::UNIFORM_BUFFER,
        BufferKind::Vertex => gl::ARRAY_BUFFER,
        BufferKind::Index => gl::ELEMENT_ARRAY_BUFFER,
    }
}

fn gl_usage(usage: BufferUsage) -> u32 {
    match usage {
        BufferUsage::Static => gl::STATIC_DRAW,
        BufferUsage::Dynamic => gl::DYNAMIC_DRAW,
        BufferUsage::Stream => gl::STREAM_DRAW,
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BufferId(NonZeroU32, BufferKind);

impl ops::Deref for BufferId {
    type Target = NonZeroU32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct BufferImpl<T> {
    __non_send: PhantomData<*mut T>,
    id: BufferId,
    bufsize: Cell<usize>,
}

pub struct Buffer<T>(ThreadGuard<BufferImpl<T>>);

impl<T> ops::Deref for Buffer<T> {
    type Target = BufferImpl<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self) {
        let id = self.0.id.get();
        unsafe { gl::DeleteBuffers(1, &id) };
    }
}

impl<T> Bind for Buffer<T> {
    type Id = BufferId;
    fn id(&self) -> Self::Id {
        self.0.id
    }

    fn bind(&self) {
        tracing::trace!(
            message = "Bind buffer",
            id = self.id().get(),
            kind = ?self.id().1
        );
        unsafe {
            gl::BindBuffer(gl_target(self.0.id.1), self.id.get());
        }
    }

    fn unbind(&self) {
        tracing::trace!(
            message = "Unbind buffer",
            id = self.id.get(),
            kind = ?self.id.1
        );
        unsafe { gl::BindBuffer(gl_target(self.0.id.1), 0) }
    }
}

impl<T: 'static + AsStd140> ApiBuffer<T> for Buffer<T> {
    type Gc = OpenGLContext;
    type Err = OpenGLError;
    type ReadBuffer<'a> = BufferSlice<'a, T>;
    type WriteBuffer<'a> = BufferSliceMut<'a, T>;

    fn len(&self) -> usize {
        self.bufsize.get() / T::std140_size_static()
    }

    fn set_data<'a>(
        &self,
        data: impl IntoIterator<Item = &'a T>,
        usage: BufferUsage,
    ) -> Result<(), Self::Err> {
        let std140 = data
            .into_iter()
            .map(|t| t.as_std140().as_bytes().to_vec())
            .flatten()
            .collect::<Vec<u8>>();
        self.bufsize.set(std140.len());
        unsafe {
            gl::BufferData(
                gl_target(self.id.1),
                std140.len() as _,
                std140.as_ptr().cast(),
                gl_usage(usage),
            )
        };
        OpenGLError::guard()?;
        Ok(())
    }

    fn slice_mut(
        &self,
        range: impl RangeBounds<usize>,
    ) -> Result<Self::WriteBuffer<'_>, Self::Err> {
        let byte_range = self.byte_range(range);
        let offset = byte_range.start;
        let size = byte_range.end - offset;
        Ok(BufferSliceMut {
            buffer: self,
            byte_range,
            data: unsafe {
                let access = gl::MAP_READ_BIT | gl::MAP_WRITE_BIT;
                let ptr = gl::MapBufferRange(gl_target(self.id.1), offset as _, size as _, access);
                OpenGLError::guard()?;
                std::slice::from_raw_parts_mut(ptr.cast(), size)
            },
        })
    }

    fn slice(&self, range: impl RangeBounds<usize>) -> Result<Self::ReadBuffer<'_>, Self::Err> {
        let byte_range = self.byte_range(range);
        let offset = byte_range.start;
        let size = byte_range.end - offset;
        Ok(BufferSlice {
            buffer: self,
            byte_range,
            data: unsafe {
                let access = gl::MAP_READ_BIT;
                let ptr = gl::MapBufferRange(gl_target(self.id.1), offset as _, size as _, access);
                OpenGLError::guard()?;
                std::slice::from_raw_parts(ptr.cast(), size as _)
            },
        })
    }
}

impl<T> Buffer<T> {
    pub(crate) fn new(kind: BufferKind) -> Self {
        let mut id = 0;
        unsafe {
            gl::GenBuffers(1, &mut id);
        }
        let inner = BufferImpl {
            __non_send: PhantomData,
            id: BufferId(NonZeroU32::new(id).unwrap(), kind),
            bufsize: Cell::new(0),
        };
        Self(ThreadGuard::new(inner))
    }
}

impl<T: 'static + AsStd140> Buffer<T> {
    fn byte_range(&self, range: impl RangeBounds<usize>) -> Range<usize> {
        let alignment = next_multiple(T::Output::ALIGNMENT, *GL_ALIGNMENT);
        let start = match range.start_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => *i + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(i) => (*i + 1).min(self.len()),
            Bound::Excluded(i) => (*i).min(self.len()),
            Bound::Unbounded => self.len(),
        } * alignment;
        start..end
    }
}

pub struct BufferSlice<'a, T: AsStd140> {
    buffer: &'a Buffer<T>,
    data: &'a [T::Output],
    byte_range: Range<usize>,
}

impl<'a, T: AsStd140> Drop for BufferSlice<'a, T> {
    fn drop(&mut self) {
        unsafe {
            gl::UnmapBuffer(gl_target(self.buffer.id.1));
        }
    }
}

impl<'a, T: AsStd140> ops::Deref for BufferSlice<'a, T> {
    type Target = [T::Output];

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, T: AsStd140> ReadBuffer<'a, T> for BufferSlice<'a, T> {
    fn len(&self) -> usize {
        self.data.len()
    }

    fn slice(&self) -> Range<usize> {
        (self.byte_range.start / T::std140_size_static())
            ..(self.byte_range.end / T::std140_size_static())
    }
}

pub struct BufferSliceMut<'a, T: AsStd140> {
    buffer: &'a Buffer<T>,
    data: &'a mut [T::Output],
    byte_range: Range<usize>,
}

impl<'a, T: AsStd140> Drop for BufferSliceMut<'a, T> {
    fn drop(&mut self) {
        unsafe {
            gl::UnmapBuffer(gl_target(self.buffer.id.1));
        }
    }
}

impl<'a, T: AsStd140> ops::Deref for BufferSliceMut<'a, T> {
    type Target = [T::Output];

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, T: AsStd140> ops::DerefMut for BufferSliceMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<'a, T: AsStd140> ReadBuffer<'a, T> for BufferSliceMut<'a, T> {
    fn len(&self) -> usize {
        self.data.len()
    }

    fn slice(&self) -> Range<usize> {
        (self.byte_range.start / T::std140_size_static())
            ..(self.byte_range.end / T::std140_size_static())
    }
}

impl<'a, T: AsStd140> WriteBuffer<'a, T> for BufferSliceMut<'a, T> {}

#[cfg(feature = "fast")]
static GL_ALIGNMENT: Lazy<NonZeroUsize> = Lazy::new(|| unsafe {
    let mut val = 0;
    gl::GetIntegerv(gl::UNIFORM_BUFFER_OFFSET_ALIGNMENT, &mut val);
    tracing::trace!("OpenGL alignment: {}", val);
    NonZeroUsize::new_unchecked(val as _)
});

#[cfg(not(feature = "fast"))]
static GL_ALIGNMENT: Lazy<NonZeroUsize> = Lazy::new(|| {
    NonZeroUsize::new(unsafe {
        let mut val = 0;
        gl::GetIntegerv(gl::UNIFORM_BUFFER_OFFSET_ALIGNMENT, &mut val);
        tracing::trace!("OpenGL alignment: {}", val);
        val as usize
    })
    .unwrap()
});

#[inline(always)]
fn next_multiple(x: usize, of: NonZeroUsize) -> usize {
    let rem = x % of.get();
    let offset = of.get() - rem;
    x + offset
}
