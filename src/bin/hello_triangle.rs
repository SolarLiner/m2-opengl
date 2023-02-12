use bytemuck::{Pod, Zeroable};
use eyre::Result;
use glam::{vec2, Vec3, Vec2, vec3};
use m2_opengl::{mesh::Mesh, Application};
use violette_low::{program::Program, framebuffer::{Framebuffer, ClearBuffer}, vertex::AsVertexAttributes, shader::{VertexShader, FragmentShader}};

#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
struct Vertex(Vec2, Vec3);

impl AsVertexAttributes for Vertex {
    type Attr = (Vec2, Vec3);
}

struct TriangleApp {
    mat_program: Program,
    mesh: Mesh<Vertex>,
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
        Ok(Self { mat_program, mesh })
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        let size = size.cast();
        Framebuffer::backbuffer().viewport(0, 0, size.width, size.height);
    }

    fn render(&mut self) {
        let frame = &*Framebuffer::backbuffer();
        frame.do_clear(ClearBuffer::COLOR).unwrap();
        self.mesh.draw(&self.mat_program, frame, false).unwrap();
    }
}

fn main() -> Result<()> {
    m2_opengl::run::<TriangleApp>("Hello Triangle")
}
