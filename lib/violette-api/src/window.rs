use std::borrow::Cow;
use cgmath::Vector2;
use crate::api::Api;

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
    type Err: Into<<Self::Api as Api>::Err>;

    fn request_redraw(&self);
    fn vsync(&self) -> bool;
    fn update(&self) -> Result<(), Self::Err>;
    fn scale_factor(&self) -> f32;
    fn physical_size(&self) -> Vector2<u32>;
}