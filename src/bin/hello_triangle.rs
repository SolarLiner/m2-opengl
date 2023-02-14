use bytemuck::{Pod, Zeroable};
use eyre::Result;
use glam::{vec2, vec3, Vec2, Vec3};
use m2_opengl::{mesh::Mesh, Application};
use violette_low::{
    framebuffer::{ClearBuffer, Framebuffer},
    program::{Program, UniformLocation},
    shader::{FragmentShader, VertexShader},
    vertex::AsVertexAttributes,
};
use winit::dpi::PhysicalSize;

#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
struct Vertex(Vec2, Vec3);

impl AsVertexAttributes for Vertex {
    type Attr = (Vec2, Vec3);
}

struct TriangleApp {
    mesh_scale: f32,
    mat_program: Program,
    uniform_scale: UniformLocation,
    mesh: Mesh<Vertex>,
    size: PhysicalSize<i32>,
}

impl Application for TriangleApp {
    fn new(size: winit::dpi::PhysicalSize<f32>) -> Result<Self> {
        let vert_shader = VertexShader::load("assets/shaders_old/triangle.vert.glsl")?;
        let frag_shader = FragmentShader::load("assets/shaders_old/triangle.frag.glsl")?;
        let mat_program = Program::new()
            .with_shader(vert_shader.id)
            .with_shader(frag_shader.id)
            .link()?;
        let mesh = Mesh::new(
            [
                Vertex(vec2(-0.5, -0.5), vec3(1., 0., 0.)),
                Vertex(vec2(0., 0.5), vec3(0., 1., 0.)),
                Vertex(vec2(0.5, -0.5), vec3(0., 0., 1.)),
            ],
            [0, 1, 2],
        )?;
        Framebuffer::backbuffer().clear_color([0., 0., 0., 1.])?;
        let uniform_scale = mat_program.uniform("scale").unwrap();
        Ok(Self {
            mesh_scale: 1.,
            mat_program,
            uniform_scale,
            mesh,
            size: size.cast(),
        })
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let size = size.cast();
        self.size = size;
        Framebuffer::backbuffer().viewport(0, 0, size.width, size.height);
    }

    fn render(&mut self) {
        let frame = &*Framebuffer::backbuffer();
        frame.viewport(0, 0, self.size.width, self.size.height);
        frame.disable_scissor().unwrap();
        frame.disable_depth_test().unwrap();
        frame.do_clear(ClearBuffer::COLOR).unwrap();
        self.mat_program.set_uniform(self.uniform_scale, self.mesh_scale).unwrap();
        self.mesh.draw(&self.mat_program, frame, false).unwrap();
    }

    fn ui(&mut self, ctx: &egui::Context) {
        egui::Window::new("Mesh options").show(ctx, |ui| {
            let scale_label = ui.label("Scale: ");
            ui.add(egui::Slider::new(&mut self.mesh_scale, 1e-2..=3.).text("Mesh scale"))
                .labelled_by(scale_label.id);
        });
    }
}

fn main() -> Result<()> {
    m2_opengl::run::<TriangleApp>("Hello Triangle")
}
