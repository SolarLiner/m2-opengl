use std::ops::{Deref, DerefMut, Range, RangeBounds};
use bytemuck::Pod;

use crate::{
    base::Resource,
    bind::Bind,
    context::GraphicsContext
};

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

pub trait Buffer<T: 'static + Pod>: Resource + Bind {
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
        data: &[T],
        usage: BufferUsage,
    ) -> Result<(), Self::Err>;
    fn slice_mut(&self, range: impl RangeBounds<usize>)
        -> Result<Self::WriteBuffer<'_>, Self::Err>;
    fn slice(&self, range: impl RangeBounds<usize>) -> Result<Self::ReadBuffer<'_>, Self::Err>;
}

pub trait ReadBuffer<'a, T: Pod>: Deref<Target = [T]> {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn slice(&self) -> Range<usize>;
}

pub trait WriteBuffer<'a, T: Pod>: ReadBuffer<'a, T> + DerefMut {}
