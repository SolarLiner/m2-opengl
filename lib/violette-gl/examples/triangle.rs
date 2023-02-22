use std::borrow::Cow;
use cgmath::{Vector2, Vector3};
use crevice::std140::{Vec2, Vec3};
use num_traits::Zero;
use violette_api::{
    api::Api,
    buffer::{
        Buffer,
        BufferKind::Vertex
    },
    context::GraphicsContext,
    window::WindowDesc
};
use violette_api::buffer::BufferUsage;
use violette_api::context::ClearBuffers;
use violette_api::framebuffer::{DrawMode, Framebuffer};
use violette_api::math::Rect;
use violette_api::shader::ShaderModule;
use violette_api::value::{ScalarType, ValueType};
use violette_api::vao::{VertexArray, VertexLayout};
use violette_api::window::Window;
use violette_gl::api::OpenGLApi;
use violette_gl::program::{Shader, ShaderType};

const VERTEX_BUFFER: [Vector2<f32>; 3] = [
    Vector2::new(-0.5, -0.5),
    Vector2::new(0.0, 0.5),
    Vector2::new(0.5, -0.5),
];

const VERTEX_SHADER: &str = r#"
#version 330 core
in vec2 pos;

void main() {
    gl_Position = vec4(pos, 0, 1);
}"#;

const FRAGMENT_SHADER: &str = r#"
#version 330 core

out vec4 color;

void main() {
    color = vec4(1, 0, 1, 1);
}"#;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let api = OpenGLApi::new();
    let window = api.clone().create_window(WindowDesc {
        title: Some(Cow::Borrowed("Hello Triangle")),
        ..Default::default()
    })?;
    let context = api.clone().create_graphics_context(window.clone())?;
    let buffer = context.create_buffer(Vertex)?;
    buffer.set_data(&VERTEX_BUFFER, BufferUsage::Static)?;
    let vao = context.create_vertex_array()?;
    vao.set_layout(std::mem::size_of::<Vec2>(), [VertexLayout {typ: ValueType::Vector(2, ScalarType::F32), offset: 0}])?;
    vao.bind_buffer(0, &buffer)?;

    let vertex_shader = Shader::with_source(ShaderType::Vertex, VERTEX_SHADER)?;
    let fragment_shader = Shader::with_source(ShaderType::Fragment, FRAGMENT_SHADER)?;
    let program = context.create_shader_module()?;
    program.add_shader_source(vertex_shader)?;
    program.add_shader_source(fragment_shader)?;
    program.link()?;

    api.run(move || {
        let frame = context.backbuffer();
        let size = window.physical_size().cast().unwrap();
        context.viewport(Rect::from_pos_size(Vector2::zero(), size));
        context.clear(ClearBuffers::COLOR);
        frame.draw_arrays(&program, &vao, DrawMode::Triangles, 3)?;
        Ok(true)
    })?;
    Ok(())
}