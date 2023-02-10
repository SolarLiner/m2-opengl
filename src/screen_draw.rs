use std::{path::Path, ops};

use anyhow::Context;
use glam::{vec2, Vec2};

use violette_low::{
    base::bindable::BindableExt,
    buffer::{Buffer, BufferKind},
    framebuffer::{BoundFB, FramebufferFeature::DepthTest, DepthTestFunction::Always},
    program::{Linked, Program},
    program::{Uniform, UniformLocation},
    vertex::DrawMode,
    vertex::{AsVertexAttributes, VertexArray},
};

#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Vertex {
    pos: Vec2,
    uv: Vec2,
}

impl AsVertexAttributes for Vertex {
    type Attr = (Vec2, Vec2);
}

const VERTICES: [Vertex; 4] = [
    Vertex {
        pos: vec2(-1., -1.),
        uv: vec2(0., 0.),
    },
    Vertex {
        pos: vec2(-1., 1.),
        uv: vec2(0., 1.),
    },
    Vertex {
        pos: vec2(1., 1.),
        uv: vec2(1., 1.),
    },
    Vertex {
        pos: vec2(1., -1.),
        uv: vec2(1., 0.),
    },
];

const INDICES: [u32; 6] = [/* Face 1: */ 0, 2, 1, /* Face 2: */ 0, 3, 2];

pub struct ScreenDraw {
    vao: VertexArray,
    indices: Buffer<u32>,
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
    pub fn new(shader_source: &str) -> anyhow::Result<Self> {
        let program = Program::from_sources(
            &std::fs::read_to_string("assets/shaders/noop.vert.glsl")?,
            Some(shader_source),
            None,
        ).context("Could not compile OpenGL shader program")?;
        let indices = Buffer::with_data(BufferKind::ElementArray, &INDICES)?;
        let mut vao = VertexArray::new();
        vao.bind()?
            .with_vertex_buffer(Buffer::with_data(BufferKind::Array, &VERTICES)?)?;
        Ok(Self {
            vao,
            indices,
            program,
        })
    }

    pub fn load(file: impl AsRef<Path>) -> anyhow::Result<Self> {
        let filename = file.as_ref().display().to_string();
        Self::new(
            std::fs::read_to_string(file)
                .context(format!("Cannot read shader from file {}", filename))?
                .as_str(),
        )
    }

    #[tracing::instrument(skip_all)]
    pub fn draw(&mut self, framebuffer: &mut BoundFB) -> anyhow::Result<()> {
        let _progbind = self.program.bind()?;
        let mut _vaobind = self.vao.bind()?;
        let idx_binding = self.indices.bind()?;
        framebuffer.disable_feature(DepthTest(Always))?;
        framebuffer.draw_elements(&mut _vaobind, &idx_binding, DrawMode::TrianglesList, ..)?;
        Ok(())
    }
}
