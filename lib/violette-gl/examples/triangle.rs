use std::{
    fs::File,
    borrow::Cow,
    time::Instant
};

use cgmath::{Vector2, Vector3};
use crevice::std140::Vec2;
use num_traits::Zero;
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

use violette_api::{
    api::Api,
    bind::Bind,
    buffer::{
        BufferUsage,
        Buffer,
        BufferKind::Vertex
    },
    context::{
        ClearBuffers,
        GraphicsContext
    },
    framebuffer::{DrawMode, Framebuffer},
    math::{Color, Rect},
    shader::ShaderModule,
    value::{ScalarType, ValueType},
    vao::{VertexArray, VertexLayout},
    window::{
        Window,
        WindowDesc
    }
};
use violette_gl::{
    program::ShaderSource,
    api::OpenGLApi,
    program::ShaderType
};

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
    vao.unbind();
    buffer.unbind();

    let program = context.create_shader_module()?;
    program.add_shader_source(ShaderSource {
        source: VERTEX_SHADER.to_string(),
        kind: ShaderType::Vertex,
    })?;
    program.add_shader_source(ShaderSource {
        source: FRAGMENT_SHADER.to_string(),
        kind: ShaderType::Fragment,
    })?;
    program.link()?;

    let uniform_color = program.uniform_location("color").unwrap();

    let start = Instant::now();
    window.clone().attach_renderer(move || {
        let (s, c) = start.elapsed().as_secs_f32().sin_cos();
        let color = Vector3::new(s, 0.5, c);
        context.backbuffer().bind();
        context.viewport(Rect::from_pos_size(
            Vector2::zero(),
            window.physical_size().cast().unwrap(),
        ));
        context.set_clear_color(Color::BLACK);
        context.clear(ClearBuffers::COLOR);
        program.bind();
        program.set_uniform(uniform_color, color);
        // vao.bind();
        buffer.bind();
        context
            .backbuffer()
            .draw_arrays(&program, &vao, DrawMode::Triangles, 3)?;
        buffer.unbind();
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
    tracing_subscriber::registry().with(fmt_layer).init();
    // .with(json_layer);
}
