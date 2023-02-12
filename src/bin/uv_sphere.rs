use std::time::Duration;

use eyre::{Context, Result};
use glam::{vec3, Quat, Vec2, Vec3};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent},
};

use m2_opengl::{material::{TextureSlot::Color, Vertex}, mesh::MeshBuilder};
use m2_opengl::{
    camera::{Camera, Projection},
    gbuffers::GeometryBuffers,
    light::LightBuffer,
    light::{GpuLight, Light},
    material::Material,
    mesh::Mesh,
    transform::Transform,
    Application,
};
use violette_low::{
    base::resource::ResourceExt,
    framebuffer::{ClearBuffer, DepthTestFunction, Framebuffer, FramebufferFeature},
    texture::Texture,
    Cull,
};

enum DebugTexture {
    Position,
    Albedo,
    Normal,
    RoughMetal,
}

struct App {
    camera: Camera,
    mesh: Mesh<Vertex>,
    lights: LightBuffer,
    geom_pass: GeometryBuffers,
    material: Material,
    dragging: bool,
    rot_target: Quat,
    last_mouse_pos: Vec2,
    debug_mode: Option<DebugTexture>,
}

impl Application for App {
    #[tracing::instrument(target = "App::new")]
    fn new(size: PhysicalSize<f32>) -> Result<Self> {
        let mesh = MeshBuilder::new(Vertex::new).uv_sphere(1.0, 32, 32)?;
        let material =
            Material::create([0.8, 0.9, 1.0], None, [0.8, 0.0])?.with_normal_amount(0.2)?;
        let lights = GpuLight::create_buffer([
            Light::Directional {
                dir: Vec3::X,
                color: Vec3::ONE * 12.,
            },
            Light::Directional {
                dir: Vec3::Z,
                color: vec3(1., 1.5, 2.),
            },
        ])?;
        let camera = Camera {
            transform: Transform::translation(vec3(0., -1., -4.)).looking_at(Vec3::ZERO),
            projection: Projection {
                width: size.width,
                height: size.height,
                ..Default::default()
            },
        };
        let mut geom_pass = GeometryBuffers::new(size.cast())?;
        geom_pass.set_exposure(0.06);
        geom_pass
            .framebuffer()
            .set_feature(FramebufferFeature::DepthTest(DepthTestFunction::Less))?;
        let rot_target = camera.transform.rotation;
        violette_low::culling(Some(Cull::Back));

        Ok(Self {
            camera,
            mesh,
            lights,
            material,
            geom_pass,
            dragging: false,
            rot_target,
            last_mouse_pos: Vec2::ONE / 2.,
            debug_mode: None,
        })
    }
    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.camera.projection.update(size.cast());
        self.geom_pass.resize(size).unwrap();
        Framebuffer::backbuffer()
            .viewport(0, 0, size.width as _, size.height as _);
    }

    fn interact(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let position = position.cast();
                let position = Vec2::new(position.x, position.y);
                if self.dragging {
                    let delta = position - self.last_mouse_pos;
                    let delta = delta * 0.01;
                    self.rot_target = Quat::from_rotation_y(delta.x)
                        * Quat::from_rotation_x(delta.y)
                        * self.rot_target;
                }
                self.last_mouse_pos = position;
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.dragging = state == ElementState::Pressed;
            }
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(code),
                        ..
                    },
                ..
            } => match code {
                VirtualKeyCode::A => self.debug_mode = Some(DebugTexture::Position),
                VirtualKeyCode::Z => self.debug_mode = Some(DebugTexture::Albedo),
                VirtualKeyCode::E => self.debug_mode = Some(DebugTexture::Normal),
                VirtualKeyCode::R => self.debug_mode = Some(DebugTexture::RoughMetal),
                _ => self.debug_mode = None,
            },
            _ => {}
        }
    }
    #[tracing::instrument(target = "App::tick", skip(self))]
    fn tick(&mut self, dt: Duration) {
        self.mesh.transform.rotation *= Quat::from_rotation_y(dt.as_secs_f32() * 0.1);
        self.camera.transform.rotation = self.camera.transform.rotation.lerp(self.rot_target, 1e-2);
    }

    #[tracing::instrument(target = "App::render", skip_all)]
    fn render(&mut self) {
        let frame = &*Framebuffer::backbuffer();
        frame.do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH).unwrap();
        // 2-pass rendering: Fill up the G-Buffers
        self.geom_pass.draw_meshes(&self.camera, &self.material, std::array::from_mut(&mut self.mesh)).unwrap();

        // 2-pass rendering: Perform defferred shading and draw to screen
        match self.debug_mode {
            None => {
                self.geom_pass
                    .draw_screen(frame, &self.camera, &self.lights)
                    .context("Cannot draw to screen")
            }
            Some(DebugTexture::Position) => {
                self.geom_pass.debug_position(frame).context("Cannot draw to screen")
            }
            Some(DebugTexture::Albedo) => {
                self.geom_pass.debug_albedo(frame).context("Cannot draw to screen")
            }
            Some(DebugTexture::Normal) => {
                self.geom_pass.debug_normal(frame).context("Cannot draw to screen")
            }
            Some(DebugTexture::RoughMetal) => {
                self.geom_pass.debug_rough_metal(frame).context("Cannot draw to screen")
            }
        }.unwrap()
    }
}

fn main() -> Result<()> {
    m2_opengl::run::<App>("UV Sphere")
}
