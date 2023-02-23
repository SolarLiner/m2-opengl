use crevice::std140::AsStd140;

use crate::{
    bind::Bind,
    context::GraphicsContext
};
use crate::base::Resource;
use crate::value::ValueType;

pub struct VertexLayout {
    pub offset: usize,
    pub typ: ValueType,
}

pub trait VertexArray: Resource + Bind {
    type Gc: GraphicsContext<VertexArray=Self>;
    type Err: Into<<Self::Gc as GraphicsContext>::Err>;

    fn set_layout(
        &self,
        stride: usize,
        layout: impl IntoIterator<IntoIter=impl ExactSizeIterator<Item=VertexLayout>>,
    ) -> Result<(), Self::Err>;
    fn bind_buffer<T: Send + Sync + AsStd140>(&self, ix: usize, buffer: &<Self::Gc as GraphicsContext>::Buffer<T>) -> Result<(), Self::Err>;
}


