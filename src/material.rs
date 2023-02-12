use std::{
    collections::BTreeSet,
    ops::Not,
    path::Path,
};

use either::Either;
use eyre::{Context, ContextCompat, Result};

use violette_low::{
    framebuffer::ClearBuffer,
    framebuffer::Framebuffer,
    gl,
    program::{Program, Uniform, UniformLocation},
    shader::Shader,
    shader::VertexShader,
    texture::{Texture, TextureUnit},
};

use crate::{camera::Camera, mesh::Mesh};

pub enum TextureSlot<const N: usize> {
    Texture(Texture<[f32; N]>),
    Color([f32; N]),
}

impl<const N: usize> From<Texture<[f32; N]>> for TextureSlot<N> {
    fn from(v: Texture<[f32; N]>) -> Self {
        Self::Texture(v)
    }
}

impl<const N: usize> From<[f32; N]> for TextureSlot<N> {
    fn from(v: [f32; N]) -> Self {
        Self::Color(v)
    }
}

pub type TextureSlotUniform<const N: usize> = Either<[f32; N], TextureUnit>;

impl<const N: usize> TextureSlot<N>
    where
        [f32; N]: Uniform,
{
    pub fn as_uniform(&self, texture_unit: u32) -> Result<TextureSlotUniform<N>> {
        Ok(match self {
            Self::Texture(texture) => {
                let unit = texture.as_uniform(texture_unit)?;
                Either::Right(unit)
            }
            &Self::Color(color) => Either::Left(color)
        })
    }
}

#[derive(Debug, Default)]
struct ShaderBuilder {
    sources: Vec<String>,
    defines: BTreeSet<String>,
    version_line: Option<String>,
}

impl ShaderBuilder {
    fn load<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.add_source(std::fs::read_to_string(path).context("I/O error")?)
    }

    fn add_source(&mut self, source: impl ToString) -> Result<()> {
        const VERSION_STR: &str = "#version";
        let source_id = self.sources.len();
        let source = source.to_string();
        let Some(first_line) = source.lines().filter_map(|s| {
            let trimmed = s.trim();
            trimmed.is_empty().not().then_some(trimmed)
        }).next() else {
            // Source is empty (or full of only whitespace), skip this source
            tracing::warn!("Source given is void of any actual source code");
            return Ok(());
        };
        if first_line.starts_with(VERSION_STR) {
            let first_line_index = source.find(first_line).unwrap();
            let rest_index = first_line_index + first_line.len();
            self.version_line.replace(first_line.to_string());
            self.sources.push(format!("#line 2 {}\n{}", source_id, &source[rest_index..]));
        } else {
            self.sources.push(format!("#line 1 {}\n{}", source_id, source));
        }
        Ok(())
    }

    fn define(&mut self, name: impl ToString) {
        self.defines.insert(name.to_string());
    }

    fn build<const K: u32>(self) -> Result<Shader<K>> {
        let source = self
            .version_line
            .into_iter()
            .chain(self.defines.into_iter().map(|v| format!("#define {}", v)))
            .chain(self.sources.into_iter())
            .reduce(|mut s, v| {
                s.push_str("\n\n");
                s.push_str(&v);
                s
            })
            .context("Empty sources")?;
        tracing::debug!(%source);
        Shader::new(&source).context("Cannot compile shader")
    }
}

pub struct Material {
    program: Program,
    uniform_color: UniformLocation,
    uniform_normal: Option<UniformLocation>,
    uniform_normal_amt: Option<UniformLocation>,
    uniform_rough_metal: UniformLocation,
    color_slot: TextureSlot<3>,
    normal_map: Option<Texture<[f32; 3]>>,
    rough_metal: TextureSlot<2>,
    normal_amount: f32,
    uniform_view_proj: UniformLocation,
    uniform_model: UniformLocation,
}

impl Material {
    pub fn create(
        color_slot: impl Into<TextureSlot<3>>,
        normal_map: impl Into<Option<Texture<[f32; 3]>>>,
        rough_metal: impl Into<TextureSlot<2>>,
    ) -> Result<Self> {
        let color_slot = color_slot.into();
        let normal_map: Option<Texture<_>> = normal_map.into();
        let rough_metal = rough_metal.into();

        let shaders_dir = Path::new("assets").join("shaders");
        let vert_shader = VertexShader::load(shaders_dir.join("mesh.vert.glsl"))?;
        let frag_shader = {
            let mut builder = ShaderBuilder::default();
            if let TextureSlot::Texture(_) = &color_slot {
                builder.define("HAS_COLOR_TEXTURE");
            }
            if normal_map.is_some() {
                builder.define("HAS_NORMAL_TEXTURE");
            }
            if let TextureSlot::Texture(_) = &rough_metal {
                builder.define("HAS_ROUGH_METAL_TEXTURE");
            }
            builder.load(shaders_dir.join("mesh.frag.glsl"))?;
            builder
                .build::<{ gl::FRAGMENT_SHADER }>()
                .context("Cannot build material shader")?
        };
        let program = Program::new().with_shader(vert_shader.id).with_shader(frag_shader.id).link()?;
        let uniform_color = program.uniform("color").unwrap();
        let uniform_normal = program.uniform("normal");
        let uniform_normal_amt = program.uniform("normal_amount");
        let uniform_rough_metal = program.uniform("rough_metal").unwrap();
        let uniform_view_proj = program.uniform("view_proj").unwrap();
        let uniform_model = program.uniform("model").unwrap();
        Ok(Self {
            program,
            uniform_color,
            uniform_normal,
            uniform_normal_amt,
            uniform_rough_metal,
            uniform_model,
            uniform_view_proj,
            color_slot,
            normal_map,
            normal_amount: 1.,
            rough_metal,
        })
    }

    pub fn with_normal_amount(mut self, amt: f32) -> Result<Material> {
        self.normal_amount = amt;
        if let Some(uniform_normal_amt) = self.uniform_normal_amt {
            self.program
                .set_uniform(uniform_normal_amt, self.normal_amount)?;
        }
        Ok(self)
    }

    pub fn draw_meshes(
        &self,
        framebuffer: &Framebuffer,
        camera: &Camera,
        meshes: &mut [Mesh],
    ) -> Result<()> {
        let mut ordering = (0..meshes.len()).collect::<Vec<_>>();
        ordering.sort_by_cached_key(|ix| meshes[*ix].distance_to_camera(camera));
        let mat_view_proj = camera.projection.matrix() * camera.transform.matrix();
        self.program.set_uniform(self.uniform_view_proj, mat_view_proj)?;
        framebuffer.do_clear(ClearBuffer::DEPTH).unwrap();


        let unit_color = self.color_slot.as_uniform(0)?;
        if let (Some(normal_map), Some(uniform_normal)) = (&self.normal_map, self.uniform_normal) {
            let unit = normal_map.as_uniform(1)?;
            self.program.set_uniform(uniform_normal, unit)?;
        }
        let unit_rough_metal = self.rough_metal.as_uniform(2)?;

        self.program.set_uniform(self.uniform_color, unit_color)?;
        self.program.set_uniform(self.uniform_rough_metal, unit_rough_metal)?;

        for mesh_ix in ordering {
            let mesh = &meshes[mesh_ix];
            self.program.set_uniform(self.uniform_model, mesh.transform.matrix())?;
            mesh.draw(&self.program, framebuffer, false)?;
        }
        Ok(())
    }
}
