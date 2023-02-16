use std::{ops, path::Path};

use eyre::{Context, Result};
use once_cell::sync::Lazy;

use violette::{
    buffer::{
        Buffer,
        ElementBuffer,
    },
    program::Program,
    vertex::{
        DrawMode,
        VertexArray,
    },
};
use violette::framebuffer::Framebuffer;

const INDICES: [u32; 6] = [/* Face 1: */ 0, 2, 1, /* Face 2: */ 0, 3, 2];
static SCREEN_INDEX_BUFFER: Lazy<ElementBuffer<u32>> = Lazy::new(|| Buffer::with_data(&INDICES).unwrap());
static SCREEN_VAO: Lazy<VertexArray> = Lazy::new(|| {
    let mut vao = VertexArray::new();
    vao.with_element_buffer(&*SCREEN_INDEX_BUFFER).unwrap();
    vao
});

#[derive(Debug)]
pub struct ScreenDraw {
    program: Program,
}

impl ops::Deref for ScreenDraw {
    type Target = Program;

    fn deref(&self) -> &Self::Target {
        &self.program
    }
}

impl ops::DerefMut for ScreenDraw {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.program
    }
}

impl ScreenDraw {
    pub fn new(shader_source: &str) -> Result<Self> {
        let program = Program::from_sources(
            &std::fs::read_to_string("assets/shaders/screen.vert.glsl")?,
            Some(shader_source),
            None,
        ).context("Could not compile OpenGL shader program")?;
        Ok(Self {
            program,
        })
    }

    pub fn load(file: impl AsRef<Path>) -> Result<Self> {
        let filename = file.as_ref().display().to_string();
        Self::new(
            std::fs::read_to_string(file)
                .context(format!("Cannot read shader from file {}", filename))?
                .as_str(),
        )
    }

    #[tracing::instrument(skip_all)]
    pub fn draw(&self, framebuffer: &Framebuffer) -> Result<()> {
        framebuffer.disable_depth_test()?;
        framebuffer.draw_elements(&self.program, &SCREEN_VAO, DrawMode::Triangles, 0..6)?;
        Ok(())
    }
}
