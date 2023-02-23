use std::borrow::Cow;
use std::fs::File;
use std::time::Instant;

use cgmath::{Vector2, Vector3};
use crevice::std140::Vec2;
use num_traits::Zero;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{EnvFilter, Layer};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use violette_api::bind::Bind;
use violette_api::buffer::BufferUsage;
use violette_api::context::ClearBuffers;
use violette_api::framebuffer::{DrawMode, Framebuffer};
use violette_api::math::{Color, Rect};
use violette_api::shader::ShaderModule;
use violette_api::value::{ScalarType, ValueType};
use violette_api::vao::{VertexArray, VertexLayout};
use violette_api::window::Window;
use violette_api::{
    api::Api,
    buffer::{Buffer, BufferKind::Vertex},
    context::GraphicsContext,
    window::WindowDesc,
};
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

uniform vec3 color;
out vec4 out_color;

void main() {
    out_color = vec4(color, 1);
}"#;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    install_tracing();
    let api = OpenGLApi::new();
    let window = api.create_window(WindowDesc {
        title: Some(Cow::Borrowed("Hello Triangle")),
        logical_size: Vector2::new(600., 600.),
        ..Default::default()
    })?;
    let context = window.context()?;
    let buffer = context.create_buffer(Vertex)?;
    buffer.bind();
    buffer.set_data(&VERTEX_BUFFER, BufferUsage::Static)?;
    let vao = context.create_vertex_array()?;
    vao.bind();
    vao.set_layout(
        std::mem::size_of::<Vec2>(),
        [VertexLayout {
            typ: ValueType::Vector(2, ScalarType::F32),
            offset: 0,
        }],
    )?;
    vao.bind_buffer(0, &buffer)?;

    let vertex_shader = Shader::with_source(ShaderType::Vertex, VERTEX_SHADER)?;
    let fragment_shader = Shader::with_source(ShaderType::Fragment, FRAGMENT_SHADER)?;
    let program = context.create_shader_module()?;
    program.add_shader_source(vertex_shader)?;
    program.add_shader_source(fragment_shader)?;
    program.link()?;

    let uniform_color = program.uniform_location("color").unwrap();

    let start = Instant::now();
    window.clone().attach_renderer(move || {
        let (s,c) = start.elapsed().as_secs_f32().sin_cos();
        let color = Vector3::new(s, 0.5, c);
        context.backbuffer().bind();
        context.viewport(Rect::from_pos_size(Vector2::zero(), window.physical_size().cast().unwrap()));
        context.set_clear_color(Color::BLACK);
        context.clear(ClearBuffers::COLOR);
        program.bind();
        program.set_uniform(uniform_color, color);
        vao.bind();
        context.backbuffer().draw_arrays(&program, &vao, DrawMode::Triangles, 3)?;
        context.swap_buffers();
        Ok(())
    });

    std::process::exit(api.run()?)
}

fn install_tracing() {
    let fmt_layer =
        tracing_subscriber::fmt::Layer::default().with_filter(EnvFilter::from_default_env());
    // let json_layer = tracing_subscriber::fmt::Layer::default()
    //     .json()
    //     .with_file(true)
    //     .with_level(true)
    //     .with_line_number(true)
    //     .with_thread_names(true)
    //     .with_thread_ids(true)
    //     .with_span_events(FmtSpan::ENTER | FmtSpan::EXIT)
    //     .with_writer(File::create("log.jsonl").unwrap());
    tracing_subscriber::registry()
        .with(fmt_layer)
        .init();
    // .with(json_layer);
}
