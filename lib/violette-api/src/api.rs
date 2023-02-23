use std::{
    sync::Arc,
    error::Error
};
use crevice::std140::AsStd140;
use crate::{
    context::GraphicsContext,
    window::{Window, WindowDesc},
};

pub trait Api: Send + Sync {
    type Err: Error;
    type Buffer<T: AsStd140>;
    type Window: Window<Api=Self>;
    type GraphicsContext: GraphicsContext<Api=Self>;

    fn create_graphics_context(
        self: Arc<Self>,
        window: Arc<Self::Window>,
    ) -> Result<Self::GraphicsContext, Self::Err>;
    fn create_window(self: Arc<Self>, desc: WindowDesc) -> Result<Arc<Self::Window>, Self::Err>;
    fn run(self: Arc<Self>, runner: impl 'static + Fn() -> Result<bool, Self::Err>) -> Result<i32, Self::Err>;
}