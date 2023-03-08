use bytemuck::{Pod, Zeroable};
use eyre::Result;
use glam::{vec2, Vec2, vec3, Vec3};

use rose_core::mesh::Mesh;
use rose_core::utils::thread_guard::ThreadGuard;
use rose_platform::{Application, PhysicalSize, RenderContext, UiContext};
use violette::{
    framebuffer::{ClearBuffer, Framebuffer},
    program::{Program, UniformLocation},
    shader::{FragmentShader, VertexShader},
};
use violette_derive::VertexAttributes;

#[derive(Debug, Default, Clone, Copy, Zeroable, Pod, VertexAttributes)]
#[repr(C)]
struct Vertex {
    pos: Vec2,
    color: Vec3,
}

struct TriangleApp {
    mesh_scale: f32,
    mat_program: ThreadGuard<Program>,
    uniform_scale: UniformLocation,
    mesh: ThreadGuard<Mesh<Vertex>>,
    size: PhysicalSize<i32>,
}

impl Application for TriangleApp {
    fn new(size: PhysicalSize<f32>, _scale_factor: f64) -> Result<Self> {
        let vert_shader = VertexShader::load("assets/shaders_old/triangle.vert.glsl")?;
        let frag_shader = FragmentShader::load("assets/shaders_old/triangle.frag.glsl")?;
        let mat_program = Program::new()
            .with_shader(vert_shader.id)
            .with_shader(frag_shader.id)
            .link()?;
        let mesh = Mesh::new(
            [
                Vertex {
                    pos: vec2(-0.5, -0.5),
                    color: vec3(1., 0., 0.),
                },
                Vertex {
                    pos: vec2(0., 0.5),
                    color: vec3(0., 1., 0.),
                },
                Vertex {
                    pos: vec2(0.5, -0.5),
                    color: vec3(0., 0., 1.),
                },
            ],
            [0, 1, 2],
        )?;
        let uniform_scale = mat_program.uniform("scale");
        Ok(Self {
            mesh_scale: 1.,
            mat_program: ThreadGuard::new(mat_program),
            uniform_scale,
            mesh: ThreadGuard::new(mesh),
            size: size.cast(),
        })
    }

    fn resize(&mut self, _size: PhysicalSize<u32>, _scale_factor: f64) -> Result<()> {
        let size = _size.cast();
        self.size = size;
        Framebuffer::viewport(0, 0, size.width, size.height);
        Ok(())
    }

    fn render(&mut self, _ctx: RenderContext) -> Result<()> {
        let frame = &*Framebuffer::backbuffer();
        Framebuffer::viewport(0, 0, self.size.width, self.size.height);
        Framebuffer::disable_scissor();
        Framebuffer::disable_depth_test();
        frame.do_clear(ClearBuffer::COLOR);
        self.mat_program
            .set_uniform(self.uniform_scale, self.mesh_scale)?;
        self.mesh.draw(&self.mat_program, frame, false)?;
        Ok(())
    }

    fn ui(&mut self, ctx: UiContext) {
        egui::Window::new("Mesh options").show(ctx.egui, |ui| {
            let scale_label = ui.label("Scale: ");
            ui.add(egui::Slider::new(&mut self.mesh_scale, 1e-2..=3.).text("Mesh scale"))
                .labelled_by(scale_label.id);
        });
    }
}

fn main() -> Result<()> {
    rose_platform::run::<TriangleApp>("Hello Triangle")
}
