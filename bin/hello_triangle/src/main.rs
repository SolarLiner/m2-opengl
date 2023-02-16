use bytemuck::{Pod, Zeroable};
use eyre::Result;
use glam::{vec2, vec3, Vec2, Vec3};

use rose_core::mesh::Mesh;
use rose_platform::{
    Application,
    PhysicalSize
};
use violette::{
    framebuffer::{ClearBuffer, Framebuffer},
    program::{Program, UniformLocation},
    shader::{FragmentShader, VertexShader},
};
use violette_derive::VertexAttributes;

#[derive(Debug, Default, Clone, Copy, Zeroable, Pod, VertexAttributes)]
#[repr(C)]
struct Vertex { pos: Vec2, color: Vec3 }


struct TriangleApp {
    mesh_scale: f32,
    mat_program: Program,
    uniform_scale: UniformLocation,
    mesh: Mesh<Vertex>,
    size: PhysicalSize<i32>,
}

impl Application for TriangleApp {
    fn new(size: PhysicalSize<f32>) -> Result<Self> {
        let vert_shader = VertexShader::load("assets/shaders_old/triangle.vert.glsl")?;
        let frag_shader = FragmentShader::load("assets/shaders_old/triangle.frag.glsl")?;
        let mat_program = Program::new()
            .with_shader(vert_shader.id)
            .with_shader(frag_shader.id)
            .link()?;
        let mesh = Mesh::new(
            [
                Vertex { pos: vec2(-0.5, -0.5), color: vec3(1., 0., 0.) },
                Vertex { pos: vec2(0., 0.5), color: vec3(0., 1., 0.) },
                Vertex { pos: vec2(0.5, -0.5), color: vec3(0., 0., 1.) },
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

    fn resize(&mut self, size: PhysicalSize<u32>) -> Result<()> {
        let size = size.cast();
        self.size = size;
        Framebuffer::backbuffer().viewport(0, 0, size.width, size.height);
        Ok(())
    }

    fn render(&mut self) -> Result<()> {
        let frame = &*Framebuffer::backbuffer();
        frame.viewport(0, 0, self.size.width, self.size.height);
        frame.disable_scissor()?;
        frame.disable_depth_test()?;
        frame.do_clear(ClearBuffer::COLOR)?;
        self.mat_program
            .set_uniform(self.uniform_scale, self.mesh_scale)?;
        self.mesh.draw(&self.mat_program, frame, false)?;
        Ok(())
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
    rose_platform::run::<TriangleApp>("Hello Triangle")
}
