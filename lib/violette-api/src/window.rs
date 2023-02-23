use std::{
    error::Error,
    borrow::Cow,
    ops::Deref,
    sync::{Arc}
};
use nalgebra_glm::TVec2 as Vector2;
use violette_input::Input;
use crate::{
    api::Api,
    context::GraphicsContext
};

pub struct WindowDesc {
    pub logical_size: Vector2<f32>,
    pub title: Option<Cow<'static, str>>,
    pub fullscreen: bool,
}

impl Default for WindowDesc {
    fn default() -> Self {
        Self {
            logical_size: Vector2::new(1600., 900.),
            title: None,
            fullscreen: false,
        }
    }
}

pub trait Window: Send + Sync {
    type Api: Api;
    type Gc: GraphicsContext<Window=Self>;
    type Err: Into<<Self::Api as Api>::Err>;
    type Input<'a>: 'a + Deref<Target=Input> where Self: 'a;

    fn attach_renderer(&self, renderer: impl 'static + Send + Sync + Fn() -> Result<(), Box<dyn Error>>);
    fn request_redraw(&self);
    fn vsync(&self) -> bool;
    fn scale_factor(&self) -> f32;
    fn physical_size(&self) -> Vector2<u32>;
    fn input<'a>(&'a self) -> Self::Input<'a>;
    fn context(self: &Arc<Self>) -> Result<Arc<Self::Gc>, Self::Err>;
    fn on_frame(&self) -> Result<(), Self::Err>;
    fn on_update(&self) -> Result<(), Self::Err>;
}