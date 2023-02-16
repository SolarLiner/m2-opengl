use std::{collections::HashMap, num::NonZeroU32};

use bytemuck::{offset_of, Pod, Zeroable};
use egui::epaint::{self, Primitive};
use eyre::Result;
use glam::{vec2, Vec2, Vec4};
use rose_core::mesh::Mesh;
use violette::{
    base::GlType,
    framebuffer::{Blend, Framebuffer},
    gl,
    program::{Program, UniformLocation},
    texture::{Texture, TextureFormat},
    vertex::{VertexAttributes, VertexDesc},
};
use winit::dpi::{LogicalPosition, PhysicalSize};

pub type UiTexture = Texture<EguiColor>;

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(transparent)]
pub struct EguiColor(pub egui::Color32);

impl GlType for EguiColor {
    const GL_TYPE: gl::types::GLenum = gl::UNSIGNED_BYTE;
    const NUM_COMPONENTS: usize = 4;
    const NORMALIZED: bool = true;
    const STRIDE: usize = std::mem::size_of::<Self>();
}

impl TextureFormat for EguiColor {
    type Subpixel = Self;
    const COUNT: usize = 1;
    const FORMAT: gl::types::GLenum = gl::RGBA;
    const INTERNAL_FORMAT: gl::types::GLenum = gl::SRGB8_ALPHA8;
    const NORMALIZED: bool = true;
}

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
pub struct UiCallback(Box<dyn 'static + Fn(egui::PaintCallbackInfo, &UiImpl) + Send + Sync>);

impl UiCallback {
    pub fn new<F: 'static + Fn(egui::PaintCallbackInfo, &UiImpl) + Send + Sync>(func: F) -> Self {
        Self(Box::new(func))
    }
}

pub struct UiImpl {
    program: Program,
    uniform_screen_size: UniformLocation,
    uniform_sampler: UniformLocation,
    mesh: Mesh<Vertex>,
    textures: HashMap<egui::TextureId, UiTexture>,
    tex_trash_bin: Vec<UiTexture>,
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
        })
    }

    pub fn draw(
        &mut self,
        frame: &Framebuffer,
        size: PhysicalSize<u32>,
        ppp: f32,
        primitives: &[egui::ClippedPrimitive],
    ) -> Result<()> {
        let size = self.prepare_painting(frame, size, ppp)?;

        for prim in primitives {
            // let (x, y, w, h) = to_gl_rect(prim.clip_rect, size.cast(), ppp);
            // frame.enable_scissor(x, y, w, h)?;

            match &prim.primitive {
                Primitive::Mesh(mesh) => {
                    self.draw_egui_mesh(frame, mesh)?;
                }
                Primitive::Callback(callback) => {
                    if callback.rect.is_positive() {
                        // let (x,y,w,h) = to_gl_rect(callback.rect, size.cast(), ppp);
                        // frame.viewport(x, y, w, h);

                        let info = egui::PaintCallbackInfo {
                            viewport: callback.rect,
                            clip_rect: prim.clip_rect,
                            pixels_per_point: ppp,
                            screen_size_px: [size.width, size.height],
                        };
                        if let Some(callback) = callback.callback.downcast_ref::<UiCallback>() {
                            (callback.0)(info, self);
                        }
                    }
                }
            }

            self.tex_trash_bin.clear();
        }
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
            .set(vertices, violette::buffer::BufferUsageHint::Dynamic)?;
        self.mesh
            .indices()
            .set(&mesh.indices, violette::buffer::BufferUsageHint::Dynamic)?;
        if let Some(texture) = self.texture(mesh.texture_id) {
            self.program
                .set_uniform(self.uniform_sampler, texture.as_uniform(0)?)?;
        }
        self.mesh.draw(&self.program, frame, false)
    }

    pub fn set_texture(&mut self, id: egui::TextureId, delta: &epaint::ImageDelta) -> Result<()> {
        let width = NonZeroU32::new(delta.image.width() as _).unwrap();
        let height = NonZeroU32::new(delta.image.height() as _).unwrap();
        let texture = self
            .textures
            .entry(id)
            .and_modify(|tex| {
                tex.clear_resize(width, height, NonZeroU32::new(1).unwrap())
                    .unwrap()
            })
            .or_insert_with(|| {
                UiTexture::new(
                    width,
                    height,
                    NonZeroU32::new(1).unwrap(),
                    violette::texture::Dimension::D2,
                )
            });
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

        match &delta.image {
            egui::ImageData::Color(image) => {
                let newtype_pixels = bytemuck::cast_slice(&image.pixels);
                texture.set_data(newtype_pixels)?;
            }
            egui::ImageData::Font(image) => {
                let pixels = image.srgba_pixels(None).map(EguiColor).collect::<Vec<_>>();
                texture.set_data(&pixels)?;
            }
        }
        Ok(())
    }

    pub fn texture(&self, id: egui::TextureId) -> Option<&UiTexture> {
        self.textures.get(&id)
    }

    pub fn insert_texture(&mut self, texture: UiTexture) -> egui::TextureId {
        let id = egui::TextureId::User(self.textures.len() as _);
        self.replace_texture(id, texture)
    }

    pub fn replace_texture(&mut self, id: egui::TextureId, texture: UiTexture) -> egui::TextureId {
        if let Some(old_texture) = self.textures.insert(id, texture) {
            self.tex_trash_bin.push(old_texture);
        }
        id
    }

    pub fn delete_texture(&mut self, id: egui::TextureId) {
        if let Some(tex) = self.textures.remove(&id) {
            self.tex_trash_bin.push(tex);
        }
    }

    fn prepare_painting(
        &self,
        frame: &Framebuffer,
        size: PhysicalSize<u32>,
        ppp: f32,
    ) -> Result<PhysicalSize<u32>> {
        violette::culling(None);
        frame.disable_depth_test()?;
        unsafe {
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        }
        frame.enable_blending(Blend::One, Blend::OneMinusSrcAlpha)?;
        let logical_size = size.to_logical::<f32>(ppp as _);
        frame.viewport(0, 0, size.width as _, size.height as _);
        self.program
            .set_uniform::<[f32; 2]>(self.uniform_screen_size, logical_size.into())?;

        Ok(size)
    }
}

fn to_gl_rect(rect: egui::Rect, size: PhysicalSize<f32>, ppp: f32) -> (i32, i32, i32, i32) {
    let min = LogicalPosition::new(rect.min.x, rect.min.y).to_physical::<f32>(ppp as _);
    let max = LogicalPosition::new(rect.max.x, rect.max.y).to_physical::<f32>(ppp as _);
    let min = Vec2::from_array(min.into())
        .clamp(Vec2::ZERO, vec2(size.width, size.height))
        .as_ivec2();
    let max = Vec2::from_array(max.into())
        .clamp(Vec2::ZERO, vec2(size.width, size.height))
        .as_ivec2();

    (min.x, min.y, max.x, max.y)
}
