use bytemuck::Pod;

use crate::{
    bind::Bind,
    context::GraphicsContext,
    base::Resource,
    value::ValueType
};
use crate::value::AsValueType;

#[derive(Debug, Copy, Clone, Hash)]
pub struct VertexLayout {
    pub offset: usize,
    pub typ: ValueType,
}

impl VertexLayout {
    pub fn from_type<T: AsValueType>(offset: usize) -> Self {
        Self {
            offset,
            typ: T::value_type(),
        }
    }
}

pub trait VertexArray: Resource + Bind {
    type Gc: GraphicsContext<VertexArray=Self>;
    type Err: Into<<Self::Gc as GraphicsContext>::Err>;

    fn set_layout(
        &self,
        stride: usize,
        layout: impl IntoIterator<IntoIter=impl ExactSizeIterator<Item=VertexLayout>>,
    ) -> Result<(), Self::Err>;
    fn bind_buffer<T: Send + Sync + Pod>(&self, ix: usize, buffer: &<Self::Gc as GraphicsContext>::Buffer<T>) -> Result<(), Self::Err>;
}


