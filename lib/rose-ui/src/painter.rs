use std::{collections::HashMap, num::NonZeroU32};

use bytemuck::{offset_of, Pod, Zeroable};
use egui::epaint::{self, Primitive};
use eyre::Result;
use glam::{vec2, IVec2, Vec2};
use winit::dpi::PhysicalSize;

use rose_core::mesh::Mesh;
use rose_core::utils::thread_guard::ThreadGuard;
use violette::{
    framebuffer::{Blend, Framebuffer},
    gl,
    program::{Program, UniformLocation},
    texture::Texture,
    vertex::{VertexAttributes, VertexDesc},
};

pub type UiTexture = Texture<f32>;

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(transparent)]
struct Vertex(epaint::Vertex);

impl VertexAttributes for Vertex {
    fn attributes() -> &'static [violette::vertex::VertexDesc] {
        vec![
            VertexDesc::from_gl_type::<Vec2>(offset_of!(epaint::Vertex, pos)),
            VertexDesc::from_gl_type::<Vec2>(offset_of!(epaint::Vertex, uv)),
            VertexDesc::from_gl_type::<[u8; 4]>(offset_of!(epaint::Vertex, color)).normalized(),
        ]
        .leak()
    }
}

#[allow(clippy::type_complexity)]
pub struct UiCallback(ThreadGuard<Box<dyn 'static + Fn(egui::PaintCallbackInfo, &UiImpl)>>);

impl UiCallback {
    pub fn new<F: 'static + Fn(egui::PaintCallbackInfo, &UiImpl)>(func: F) -> Self {
        Self(ThreadGuard::new(Box::new(func)))
    }
}

pub struct UiImpl {
    program: Program,
    uniform_screen_size: UniformLocation,
    uniform_sampler: UniformLocation,
    mesh: Mesh<Vertex>,
    textures: HashMap<egui::TextureId, UiTexture>,
    tex_trash_bin: Vec<UiTexture>,
    current_fbo: Option<*const Framebuffer>,
}

impl UiImpl {
    pub fn new() -> Result<Self> {
        let program = Program::load(
            "assets/shaders/ui.vert.glsl",
            Some("assets/shaders/ui.frag.glsl"),
            None::<&'static str>,
        )?;
        let uniform_screen_size = program.uniform("u_screen_size").unwrap();
        let uniform_sampler = program.uniform("u_sampler").unwrap();
        let mesh = Mesh::empty()?;
        Ok(Self {
            program,
            uniform_screen_size,
            uniform_sampler,
            mesh,
            textures: HashMap::new(),
            tex_trash_bin: Vec::default(),
            current_fbo: None,
        })
    }

    #[tracing::instrument(skip_all)]
    pub fn draw(
        &mut self,
        frame: &Framebuffer,
        size: PhysicalSize<u32>,
        ppp: f32,
        primitives: &[egui::ClippedPrimitive],
    ) -> Result<()> {
        tracing::trace!(message="Egui draw", primitices=%primitives.len());
        self.current_fbo.replace(frame);
        let _ = self.prepare_painting(size, ppp)?;
        let sizef = size.cast();

        for prim in primitives {
            let (x, y, w, h) = to_gl_rect(prim.clip_rect, sizef, ppp);
            Framebuffer::enable_scissor(x, y, w, h);

            match &prim.primitive {
                Primitive::Mesh(mesh) => {
                    self.draw_egui_mesh(frame, mesh)?;
                }
                Primitive::Callback(callback) => {
                    if callback.rect.is_positive() {
                        let (x, y, w, h) = to_gl_rect(callback.rect, sizef, ppp);
                        Framebuffer::viewport(x, y, w, h);

                        let info = egui::PaintCallbackInfo {
                            viewport: callback.rect,
                            clip_rect: prim.clip_rect,
                            pixels_per_point: ppp,
                            screen_size_px: [size.width, size.height],
                        };
                        if let Some(callback) = callback.callback.downcast_ref::<UiCallback>() {
                            if let Some(cb) = callback.0.get() {
                                cb(info, self);
                            } else {
                                tracing::error!("Ui painter callback not created within the render thread -- this cannot work as OpenGL is not multithreaded")
                            }
                        }
                        Framebuffer::viewport(0, 0, size.width as _, size.height as _);
                    }
                }
            }
        }
        Framebuffer::disable_scissor();
        Framebuffer::viewport(0, 0, size.width as _, size.height as _);
        self.tex_trash_bin.clear();
        self.current_fbo.take();
        Ok(())
    }

    pub fn draw_egui_mesh(&mut self, frame: &Framebuffer, mesh: &egui::Mesh) -> Result<()> {
        eyre::ensure!(mesh.is_valid(), "Egui mesh must be valid");
        if mesh.is_empty() {
            return Ok(());
        }

        let vertices = bytemuck::cast_slice(&mesh.vertices);
        self.mesh
            .vertices()
            .set(vertices, violette::buffer::BufferUsageHint::Stream)?;
        self.mesh
            .indices()
            .set(&mesh.indices, violette::buffer::BufferUsageHint::Stream)?;
        if let Some(texture) = self.texture(mesh.texture_id) {
            self.program
                .set_uniform(self.uniform_sampler, texture.as_uniform(0)?)?;
        }
        self.mesh.draw(&self.program, frame, false)
    }

    pub fn set_texture(&mut self, id: egui::TextureId, delta: &epaint::ImageDelta) -> Result<()> {
        tracing::trace!(message = "Set texture from delta", ?id);
        let width = NonZeroU32::new(delta.image.width() as _).unwrap();
        let height = NonZeroU32::new(delta.image.height() as _).unwrap();

        let pixels = match &delta.image {
            egui::ImageData::Color(image) => {
                let newtype_pixels = bytemuck::cast_slice(&image.pixels).to_vec();
                newtype_pixels
            }
            egui::ImageData::Font(image) => image.pixels.clone(),
        };
        if let Some(texture) = self.textures.get_mut(&id) {
            if let Some(pos) = delta.pos {
                let pos = IVec2::from_array(pos.map(|x| x as _));
                let size = IVec2::from_array(delta.image.size().map(|x| x as _));
                texture.set_sub_data_2d(0, pos.x, pos.y, size.x, size.y, &pixels)?;
            } else {
                tracing::debug!(
                    "Reset image {:?} to {}x{} with [_; {}] pixels",
                    id,
                    width.get(),
                    height.get(),
                    pixels.len()
                );
                texture.clear_resize(width, height, unsafe { NonZeroU32::new_unchecked(1) })?;
                texture.set_data(&pixels)?;
            }
        } else {
            self.textures.insert(id, {
                let texture = UiTexture::from_2d_pixels(width, &pixels)?;
                texture.filter_min(match delta.options.minification {
                    egui::TextureFilter::Nearest => violette::texture::SampleMode::Nearest,
                    egui::TextureFilter::Linear => violette::texture::SampleMode::Linear,
                })?;
                texture.filter_mag(match delta.options.minification {
                    egui::TextureFilter::Nearest => violette::texture::SampleMode::Nearest,
                    egui::TextureFilter::Linear => violette::texture::SampleMode::Linear,
                })?;
                texture.wrap_s(violette::texture::TextureWrap::ClampEdge)?;
                texture.wrap_t(violette::texture::TextureWrap::ClampEdge)?;
                texture
            });
        }
        Ok(())
    }

    pub fn texture(&self, id: egui::TextureId) -> Option<&UiTexture> {
        self.textures.get(&id)
    }

    pub fn insert_texture(&mut self, texture: UiTexture) -> egui::TextureId {
        let id = egui::TextureId::User(self.textures.len() as _);
        tracing::trace!(message = "Insert texture", ?id);
        self.replace_texture(id, texture)
    }

    pub fn replace_texture(&mut self, id: egui::TextureId, texture: UiTexture) -> egui::TextureId {
        tracing::trace!(message = "Replace texture", ?id);
        if let Some(old_texture) = self.textures.insert(id, texture) {
            self.tex_trash_bin.push(old_texture);
        }
        id
    }

    pub fn delete_texture(&mut self, id: egui::TextureId) {
        tracing::trace!(message = "Delete texture", ?id);
        if let Some(tex) = self.textures.remove(&id) {
            self.tex_trash_bin.push(tex);
        }
    }

    pub fn framebuffer(&self) -> &Framebuffer {
        // Safety: This is valid because the FBO is removed at the end of the draw call
        unsafe { &*self.current_fbo.unwrap() }
    }

    fn prepare_painting(&self, size: PhysicalSize<u32>, ppp: f32) -> Result<PhysicalSize<u32>> {
        violette::culling(None);
        Framebuffer::disable_depth_test();
        unsafe {
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        }
        Framebuffer::enable_blending(Blend::One, Blend::OneMinusSrcAlpha);
        let logical_size = size.to_logical::<f32>(ppp as _);
        Framebuffer::viewport(0, 0, size.width as _, size.height as _);
        self.program
            .set_uniform::<[f32; 2]>(self.uniform_screen_size, logical_size.into())?;

        Ok(size)
    }
}

fn to_gl_rect(rect: egui::Rect, size: PhysicalSize<f32>, ppp: f32) -> (i32, i32, i32, i32) {
    // let min = LogicalPosition::new(rect.min.x, rect.min.y).to_physical::<f32>(ppp as _);
    // let max = LogicalPosition::new(rect.max.x, rect.max.y).to_physical::<f32>(ppp as _);
    // let min = Vec2::from_array(min.into())
    //     .clamp(Vec2::ZERO, vec2(size.width, size.height))
    //     .as_ivec2();
    // let max = Vec2::from_array(max.into())
    //     .clamp(Vec2::ZERO, vec2(size.width, size.height))
    //     .as_ivec2();

    // (min.x, min.y, max.x, max.y)
    let pos = Vec2::from_array(rect.left_bottom().into()) * ppp;
    let bias = Vec2::Y * size.height;
    let scale = vec2(1., -1.);
    let pos = pos * scale + bias;
    let size = Vec2::from_array(rect.size().into()) * ppp;

    let pos = pos.as_ivec2();
    let size = size.as_ivec2();
    (pos.x, pos.y, size.x, size.y)
}
