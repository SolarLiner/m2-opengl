use std::{
    f32::consts::{PI, TAU},
    time::Duration,
};

use eyre::{Context, Result};
use glam::{vec2, vec3, Mat3, Quat, Vec2, Vec3, UVec2};

use rose_core::{
    camera::Camera,
    gbuffers::GeometryBuffers,
    light::{Light, LightBuffer, GpuLight},
    material::{Material, Vertex},
    mesh::{Mesh, MeshBuilder},
    postprocess::Postprocess,
    transform::Transform,
};
use rose_core::camera::Projection;
use rose_platform::{
    events::{
        ElementState, KeyboardInput, ModifiersState, MouseButton, MouseScrollDelta, VirtualKeyCode,
        WindowEvent,
    },
    Application, PhysicalSize,
};
use violette::{
    Cull,
    framebuffer::Framebuffer,
    texture::Texture,
    framebuffer::{ClearBuffer, DepthTestFunction}
};

mod camera_controller;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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
    ctrl_pressed: bool,
    dragging: Option<MouseButton>,
    last_mouse_pos: Vec2,
    debug_mode: Option<DebugTexture>,
    exposure: f32,
    camera_controller: camera_controller::OrbitCameraController,
}

impl Application for App {
    #[tracing::instrument(target = "App::new")]
    fn new(size: PhysicalSize<f32>) -> Result<Self> {
        let mesh = MeshBuilder::new(Vertex::new).uv_sphere(1.0, 32, 32)?;
        let material = Material::create(
            Texture::load_rgb32f("assets/textures/moon_color.png")?,
            Texture::load_rgb32f("assets/textures/moon_normal.png")?,
            [0.8, 0.0],
        )?
        .with_normal_amount(0.1)?;
        let lights = GpuLight::create_buffer([
            Light::Ambient {
                color: Vec3::ONE * 0.01,
            },
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
        let size = UVec2::from_array(size.cast::<u32>().into());
        let geom_pass = GeometryBuffers::new(size)?;
        let post_process = Postprocess::new(size)?;
        post_process.set_exposure(1e-3)?;
        post_process.framebuffer().clear_color([0., 0., 0., 1.])?;
        post_process.framebuffer().clear_depth(1.)?;

        let geo_fbo = geom_pass.framebuffer();
        geo_fbo.enable_depth_test(DepthTestFunction::Less)?;
        geo_fbo.clear_color([0., 0., 0., 1.])?;
        geo_fbo.clear_depth(1.)?;
        violette::culling(Some(Cull::Back));

        let sizei = size.as_ivec2();
        Framebuffer::backbuffer().viewport(0, 0, sizei.x, sizei.y);

        Ok(Self {
            exposure: 1e-3,
            camera,
            mesh,
            lights,
            material,
            geom_pass,
            post_process,
            dragging: None,
            ctrl_pressed: false,
            last_mouse_pos: Vec2::ONE / 2.,
            debug_mode: None,
            camera_controller: camera_controller::OrbitCameraController::default(),
        })
    }
    fn resize(&mut self, size: PhysicalSize<u32>) -> Result<()> {
        let sizei = size.cast();
        let size = UVec2::from_array(size.into());
        let sizef = size.as_vec2();
        self.camera.projection.update(sizef);
        self.geom_pass.resize(size)?;
        self.post_process.resize(size)?;
        Framebuffer::backbuffer().viewport(0, 0, sizei.width, sizei.height);
        Ok(())
    }

    fn interact(&mut self, event: WindowEvent) -> Result<()> {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let position = position.cast();
                let position = Vec2::new(position.x, position.y);
                match self.dragging {
                    Some(MouseButton::Left) => self
                        .camera_controller
                        .orbit(&self.camera, position - self.last_mouse_pos),
                    Some(MouseButton::Right) => self
                        .camera_controller
                        .pan(&self.camera, position - self.last_mouse_pos),
                    _ => {}
                }
                self.last_mouse_pos = position;
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if state == ElementState::Pressed {
                    self.dragging = match button {
                        MouseButton::Right | MouseButton::Left if self.ctrl_pressed => {
                            Some(MouseButton::Right)
                        }
                        MouseButton::Left => Some(MouseButton::Left),
                        _ => None,
                    }
                } else {
                    self.dragging.take();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(_, y) => self.camera_controller.scroll(&self.camera, y),
                MouseScrollDelta::PixelDelta(delta) => {
                    self.camera_controller.scroll(&self.camera, delta.y as _)
                }
            },
            WindowEvent::ModifiersChanged(state) => {
                self.ctrl_pressed = state.contains(ModifiersState::CTRL)
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
                VirtualKeyCode::T => self.debug_mode = None,
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }
    #[tracing::instrument(target = "App::tick", skip(self))]
    fn tick(&mut self, dt: Duration) -> Result<()> {
        self.camera_controller.update(dt, &mut self.camera);
        Ok(())
    }

    #[cfg(never)]
    #[tracing::instrument(target = "App::render", skip_all)]
    fn render(&mut self) {
        let frame = &*Framebuffer::backbuffer();
        frame.clear_color([0., 0., 0., 1.]).unwrap();
        frame.clear_depth(1.).unwrap();
        frame
            .enable_features(FramebufferFeatureId::DEPTH_TEST)
            .unwrap();
        frame
            .set_feature(FramebufferFeature::DepthTest(DepthTestFunction::Less))
            .unwrap();
        frame
            .do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH)
            .unwrap();
        self.material
            .draw_meshes(frame, &self.camera, std::array::from_mut(&mut self.mesh))
            .unwrap();
    }

    #[tracing::instrument(target = "App::render", skip_all)]
    fn render(&mut self) -> Result<()> {
        let backbuffer = &Framebuffer::backbuffer();
        backbuffer.disable_scissor().unwrap();
        backbuffer.do_clear(ClearBuffer::COLOR)?;

        // 2-pass rendering: Fill up the G-Buffers
        self.geom_pass
            .draw_meshes(
                &self.camera,
                &self.material,
                std::array::from_mut(&mut self.mesh),
            )
            ?;

        // 2-pass rendering: Perform defferred shading and draw to screen
        match self.debug_mode {
            None => {
                let frame = self.post_process.framebuffer();
                frame
                    .do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH)?;
                self.geom_pass
                    .draw_screen(frame, &self.camera, &self.lights)
                    .context("Cannot draw to screen")
                    ?;

                // Post-processing
                self.post_process.draw(backbuffer)
            }
            Some(DebugTexture::Position) => self
                .geom_pass
                .debug_position(&Framebuffer::backbuffer())
                .context("Cannot draw to screen"),
            Some(DebugTexture::Albedo) => self
                .geom_pass
                .debug_albedo(&Framebuffer::backbuffer())
                .context("Cannot draw to screen"),
            Some(DebugTexture::Normal) => self
                .geom_pass
                .debug_normal(&Framebuffer::backbuffer())
                .context("Cannot draw to screen"),
            Some(DebugTexture::RoughMetal) => self
                .geom_pass
                .debug_rough_metal(&Framebuffer::backbuffer())
                .context("Cannot draw to screen"),
        }?;
        Ok(())
    }

    fn ui(&mut self, ctx: &egui::Context) {
        egui::Window::new("Camera controls").show(ctx, |ui| {
            self.camera_controller.ui(ui);
            let exposure_label = ui.label("Exposure:");
            if ui
                .add(
                    egui::Slider::new(&mut self.exposure, 1e-6..=10.)
                        .logarithmic(true)
                        .show_value(true)
                        .custom_formatter(|v, _| format!("{:+1.1} EV", v.log2()))
                        .text("Exposure"),
                )
                .labelled_by(exposure_label.id)
                .changed()
            {
                self.post_process.set_exposure(self.exposure).unwrap();
            }
        });
        egui::Window::new("Debug textures").show(ctx, |ui| {
            if ui
                .add(egui::RadioButton::new(self.debug_mode.is_none(), "None"))
                .clicked()
            {
                self.debug_mode.take();
            }
            if ui
                .add(egui::RadioButton::new(
                    self.debug_mode == Some(DebugTexture::Position),
                    "Position",
                ))
                .clicked()
            {
                self.debug_mode.replace(DebugTexture::Position);
            }
            if ui
                .add(egui::RadioButton::new(
                    self.debug_mode == Some(DebugTexture::Albedo),
                    "Albedo",
                ))
                .clicked()
            {
                self.debug_mode.replace(DebugTexture::Albedo);
            }
            if ui
                .add(egui::RadioButton::new(
                    self.debug_mode == Some(DebugTexture::Normal),
                    "Normal",
                ))
                .clicked()
            {
                self.debug_mode.replace(DebugTexture::Normal);
            }
            if ui
                .add(egui::RadioButton::new(
                    self.debug_mode == Some(DebugTexture::RoughMetal),
                    "Roughness / Metal",
                ))
                .clicked()
            {
                self.debug_mode.replace(DebugTexture::RoughMetal);
            }
        });
    }
}

fn main() -> Result<()> {
    rose_platform::run::<App>("UV Sphere")
}
