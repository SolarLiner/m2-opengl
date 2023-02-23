use std::ops::{Deref, DerefMut, Range, RangeBounds};

use crevice::std140::AsStd140;
use crate::base::Resource;
use crate::bind::Bind;

use crate::context::GraphicsContext;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BufferKind {
    Vertex,
    Index,
    UniformBlock,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BufferUsage {
    Static,
    Dynamic,
    Stream,
}

// TODO: Relax restriction on [`AsStd140`]
pub trait Buffer<T: 'static + AsStd140>: Resource + Bind {
    type Gc: GraphicsContext;
    type Err: Into<<Self::Gc as GraphicsContext>::Err>;
    type ReadBuffer<'a>: ReadBuffer<'a, T> where Self: 'a;
    type WriteBuffer<'a>: WriteBuffer<'a, T> where Self: 'a;

    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn set_data<'a>(
        &self,
        data: impl IntoIterator<Item = &'a T>,
        usage: BufferUsage,
    ) -> Result<(), Self::Err>;
    fn slice_mut(&self, range: impl RangeBounds<usize>)
        -> Result<Self::WriteBuffer<'_>, Self::Err>;
    fn slice(&self, range: impl RangeBounds<usize>) -> Result<Self::ReadBuffer<'_>, Self::Err>;
}

pub trait ReadBuffer<'a, T: AsStd140>: Deref<Target = [T::Output]> {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn slice(&self) -> Range<usize>;
}

pub trait WriteBuffer<'a, T: AsStd140>: ReadBuffer<'a, T> + DerefMut {}
