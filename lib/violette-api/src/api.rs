use std::{
    sync::Arc,
    error::Error
};

use crate::{
    window::{Window, WindowDesc},
};

pub trait Api: Send + Sync {
    type Err: Error;
    type Window: Window<Api=Self>;

    fn create_window(self: &Arc<Self>, desc: WindowDesc) -> Result<Arc<Self::Window>, Self::Err>;
    fn run(self: &Arc<Self>) -> Result<i32, Self::Err>;
}