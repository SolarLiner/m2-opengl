// main.rs

use rose_platform::{Application, PhysicalSize, RenderContext};

use crate::tests::IntegrationTest;

pub mod tests;

struct TestRunner(PhysicalSize<f32>);

impl Application for TestRunner {
    fn new(size: PhysicalSize<f32>) -> eyre::Result<Self> {
        Ok(Self(size))
    }

    fn render(&mut self, mut ctx: RenderContext) -> eyre::Result<()> {
        for test in inventory::iter::<IntegrationTest> {
            (test.test_fn)(self.0);
        }
        ctx.quit();
        Ok(())
    }
}

fn main() {
    std::env::set_var(
        "RUST_LOG",
        std::env::var("RUST_LOG").as_deref().unwrap_or("info"),
    );
    rose_platform::run::<TestRunner>("Test runner").unwrap();
}
