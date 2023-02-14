use std::time::Duration;

use eyre::{Context, Result};
use glam::{vec3, Quat, Vec2, Vec3};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent},
};

use m2_opengl::{material::{Vertex}, mesh::MeshBuilder, postprocess::Postprocess};
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
    framebuffer::{ClearBuffer, DepthTestFunction, Framebuffer},
    Cull, texture::Texture,
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
    post_process: Postprocess,
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
            Material::create(Texture::load_rgb32f("assets/textures/moon_color.png")?, Texture::load_rgb32f("assets/textures/moon_normal.png")?, [0.8, 0.0])?.with_normal_amount(0.1)?;
        let lights = GpuLight::create_buffer([
            Light::Ambient { color: Vec3::ONE * 0.01 },
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
        let geom_pass = GeometryBuffers::new(size.cast())?;
        let post_process = Postprocess::new(size.cast())?;
        post_process.set_exposure(1e-3)?;
        post_process.framebuffer().clear_color([0., 0., 0., 1.])?;
        post_process.framebuffer().clear_depth(1.)?;

        let geo_fbo = geom_pass
            .framebuffer();
        geo_fbo.enable_depth_test(DepthTestFunction::Less)?;
        geo_fbo.clear_color([0., 0., 0., 1.])?;
        geo_fbo.clear_depth(1.)?;

        let rot_target = camera.transform.rotation;
        violette_low::culling(Some(Cull::Back));

        let size = size.cast();
        Framebuffer::backbuffer().viewport(0, 0, size.width, size.height);

        Ok(Self {
            camera,
            mesh,
            lights,
            material,
            geom_pass,
            post_process,
            dragging: false,
            rot_target,
            last_mouse_pos: Vec2::ONE / 2.,
            debug_mode: None,
        })
    }
    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.camera.projection.update(size.cast());
        self.geom_pass.resize(size).unwrap();
        self.post_process.resize(size).unwrap();
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

    #[cfg(never)]
    #[tracing::instrument(target = "App::render", skip_all)]
    fn render(&mut self) {
        let frame = &*Framebuffer::backbuffer();
        frame.clear_color([0., 0., 0., 1.]).unwrap();
        frame.clear_depth(1.).unwrap();
        frame.enable_features(FramebufferFeatureId::DEPTH_TEST).unwrap();
        frame.set_feature(FramebufferFeature::DepthTest(DepthTestFunction::Less)).unwrap();
        frame.do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH).unwrap();
        self.material.draw_meshes(frame, &self.camera, std::array::from_mut(&mut self.mesh)).unwrap();
    }

    #[tracing::instrument(target = "App::render", skip_all)]
    fn render(&mut self) {
        let backbuffer = &Framebuffer::backbuffer();
        backbuffer.do_clear(ClearBuffer::COLOR|ClearBuffer::DEPTH).unwrap();

        // 2-pass rendering: Fill up the G-Buffers
        self.geom_pass.draw_meshes(&self.camera, &self.material, std::array::from_mut(&mut self.mesh)).unwrap();

        // 2-pass rendering: Perform defferred shading and draw to screen
        match self.debug_mode {
            None => {
                let frame = self.post_process.framebuffer();
                frame.do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH).unwrap();
                self.geom_pass
                    .draw_screen(frame, &self.camera, &self.lights)
                    .context("Cannot draw to screen").unwrap();

                // Post-processing
                self.post_process.draw(backbuffer)
            }
            Some(DebugTexture::Position) => {
                self.geom_pass.debug_position(&Framebuffer::backbuffer()).context("Cannot draw to screen")
            }
            Some(DebugTexture::Albedo) => {
                self.geom_pass.debug_albedo(&Framebuffer::backbuffer()).context("Cannot draw to screen")
            }
            Some(DebugTexture::Normal) => {
                self.geom_pass.debug_normal(&Framebuffer::backbuffer()).context("Cannot draw to screen")
            }
            Some(DebugTexture::RoughMetal) => {
                self.geom_pass.debug_rough_metal(&Framebuffer::backbuffer()).context("Cannot draw to screen")
            }
        }.unwrap();
    }


}

fn main() -> Result<()> {
    m2_opengl::run::<App>("UV Sphere")
}
