use rose_platform::PhysicalSize;

pub mod violette;

#[derive(Debug)]
pub struct IntegrationTest {
    pub name: &'static str,
    pub test_fn: fn(PhysicalSize<f32>),
}

inventory::collect!(IntegrationTest);
