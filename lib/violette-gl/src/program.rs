use std::{
    fmt::Formatter,
    hash::{Hash, Hasher},
    ffi::{CStr, CString},
    fmt,
    num::NonZeroU32,
    rc::Rc
};


use violette_api::math::*;
use dashmap::DashSet;
use duplicate::duplicate_item;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use violette_api::{
    base::Resource,
    bind::Bind,
    shader::{AsUniform, ShaderModule}
};


use crate::{api::GlErrorKind, api::OpenGLError, context::OpenGLContext, get_ext_label, set_ext_label, Gl, GlObject};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, FromPrimitive)]
#[repr(u32)]
pub enum ShaderType {
    Vertex = gl::VERTEX_SHADER,
    Fragment = gl::FRAGMENT_SHADER,
    Geometry = gl::GEOMETRY_SHADER,
}

pub struct Program {
    gl: Gl,
    id: NonZeroU32,
    shaders: DashSet<Shader>,
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe { self.gl.DeleteShader(self.id.get()) }
    }
}

impl fmt::Debug for Program {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Program").field(&self.id.get()).finish()
    }
}

impl Program {
    pub fn new(gl: &Gl) -> Result<Self, OpenGLError> {
        Ok(Self {
            gl: gl.clone(),
            id: unsafe {
                NonZeroU32::new(gl.CreateProgram()).ok_or_else(|| {
                    OpenGLError::from(
                        GlErrorKind::current_error(gl).unwrap_or(GlErrorKind::UnknownError),
                    )
                })?
            },
            shaders: DashSet::new(),
        })
    }
}

impl Bind for Program {
    type Id = NonZeroU32;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn bind(&self) {
        unsafe {
            self.gl.UseProgram(self.id.get());
        }
    }

    fn unbind(&self) {
        unsafe {
            self.gl.UseProgram(0);
        }
    }
}

struct Shader {
    gl: Gl,
    id: NonZeroU32,
}

impl fmt::Debug for Shader {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Shader").field(&self.id.get()).finish()
    }
}

impl Hash for Shader {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for Shader {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Shader {}

impl Shader {
    pub fn with_source(gl: &Gl, typ: ShaderType, source: &str) -> Result<Self, OpenGLError> {
        let this = Self::new(gl, typ)?;
        this.add_source(source);
        this.compile()?;
        Ok(this)
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.gl.DeleteShader(self.id.get());
        }
    }
}

impl GlObject for Shader {
    const GL_NAME: gl::types::GLenum = gl::SHADER_OBJECT_EXT;

    fn gl(&self) -> &Gl {
        &self.gl
    }

    fn id(&self) -> u32 {
        self.id.get()
    }
}

impl Resource for Shader {
    fn set_name(&self, name: impl ToString) {
        set_ext_label(self, name)
    }

    fn get_name(&self) -> Option<String> {
        get_ext_label(self)
    }
}

impl Shader {
    pub fn new(gl: &Gl, typ: ShaderType) -> Result<Self, OpenGLError> {
        Ok(Self {
            gl: gl.clone(),
            id: unsafe {
                NonZeroU32::new(gl.CreateShader(typ as _)).ok_or_else(|| {
                    OpenGLError::from(
                        GlErrorKind::current_error(gl).unwrap_or(GlErrorKind::UnknownError),
                    )
                })?
            },
        })
    }

    pub fn add_source(&self, source: &str) {
        let source = CString::new(source).unwrap();
        let source_len = source.as_bytes().len();
        unsafe {
            let lengths = [source_len as gl::types::GLint];
            self.gl.ShaderSource(
                self.id.get(),
                1,
                &source.as_c_str().as_ptr(),
                lengths.as_ptr(),
            );
        }
    }

    pub fn compile(&self) -> Result<(), OpenGLError> {
        let status = unsafe {
            self.gl.CompileShader(self.id.get());
            let mut status = 0;
            self.gl.GetShaderiv(self.id.get(), gl::COMPILE_STATUS, &mut status);
            status as gl::types::GLboolean == gl::TRUE
        };
        if !status {
            let info_log = unsafe {
                let mut buf = vec![0; 2048];
                let mut len = 0;
                self.gl.GetShaderInfoLog(self.id.get(), 2048, &mut len, buf.as_mut_ptr().cast());
                CStr::from_bytes_with_nul(&buf[..(len + 1) as _])
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            };
            Err(OpenGLError::with_info_log(&self.gl, info_log))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub enum Uniform {
    Int(i32),
    Uint(u32),
    Float(f32),
    Ivec2([i32; 2]),
    UIvec2([u32; 2]),
    Vec2([f32; 2]),
    Ivec3([i32; 3]),
    UIvec3([u32; 3]),
    Vec3([f32; 3]),
    Ivec4([i32; 4]),
    UIvec4([u32; 4]),
    Vec4([f32; 4]),
    Mat2([[f32; 2]; 2]),
    Mat3([[f32; 3]; 3]),
    Mat4([[f32; 4]; 4]),
    Mat23([[f32; 2]; 3]),
    Mat24([[f32; 2]; 4]),
    Mat32([[f32; 3]; 2]),
    Mat34([[f32; 3]; 4]),
    Mat42([[f32; 4]; 2]),
    Mat43([[f32; 4]; 3]),
    SliceInt(Rc<[i32]>),
    SliceUint(Rc<[u32]>),
    SliceFloat(Rc<[f32]>),
}

#[duplicate_item(
ty      vec        uniform_ty;
[i32]   [Vec2]     [Ivec2];
[u32]   [Vec2]     [UIvec2];
[f32]   [Vec2]     [Vec2];
[i32]   [Vec3]     [Ivec3];
[u32]   [Vec3]     [UIvec3];
[f32]   [Vec3]     [Vec3];
[i32]   [Vec4]     [Ivec4];
[u32]   [Vec4]     [UIvec4];
[f32]   [Vec4]     [Vec4];
)]
impl From<vec<ty>> for Uniform {
    fn from(value: vec<ty>) -> Self {
        Uniform::uniform_ty(value.into())
    }
}

#[duplicate_item(
mat         mat_uniform;
[Mat2]      [Mat2];
[Mat2x3]    [Mat23];
[Mat2x4]    [Mat24];
[Mat3x2]    [Mat32];
[Mat3]      [Mat3];
[Mat3x4]    [Mat34];
[Mat4x2]    [Mat42];
[Mat4x3]    [Mat43];
[Mat4]      [Mat4];
)]
impl From<mat<f32>> for Uniform {
    fn from(value: mat<f32>) -> Self {
        Uniform::mat_uniform(value.into())
    }
}

impl FromPrimitive for Uniform {
    fn from_isize(_n: isize) -> Option<Self> {
        None
    }

    fn from_i8(n: i8) -> Option<Self> {
        Some(Self::Int(n as _))
    }

    fn from_i16(n: i16) -> Option<Self> {
        Some(Self::Int(n as _))
    }

    fn from_i32(n: i32) -> Option<Self> {
        Some(Self::Int(n as _))
    }

    fn from_i64(n: i64) -> Option<Self> {
        Some(Self::Int(n as _))
    }

    fn from_i128(n: i128) -> Option<Self> {
        Some(Self::Int(n as _))
    }

    fn from_usize(n: usize) -> Option<Self> {
        Some(Self::Uint(n as _))
    }

    fn from_u8(n: u8) -> Option<Self> {
        Some(Self::Uint(n as _))
    }

    fn from_u16(n: u16) -> Option<Self> {
        Some(Self::Uint(n as _))
    }

    fn from_u32(n: u32) -> Option<Self> {
        Some(Self::Uint(n))
    }

    fn from_u64(n: u64) -> Option<Self> {
        Some(Self::Uint(n as _))
    }

    fn from_u128(n: u128) -> Option<Self> {
        Some(Self::Uint(n as _))
    }

    fn from_f32(n: f32) -> Option<Self> {
        Some(Self::Float(n))
    }

    fn from_f64(n: f64) -> Option<Self> {
        Some(Self::Float(n as _))
    }
}

impl GlObject for Program {
    const GL_NAME: gl::types::GLenum = gl::PROGRAM_OBJECT_EXT;

    fn gl(&self) -> &Gl {
        &self.gl
    }

    fn id(&self) -> u32 {
        self.id.get()
    }
}

impl Resource for Program {
    fn set_name(&self, name: impl ToString) {
        set_ext_label(self, name)
    }

    fn get_name(&self) -> Option<String> {
        get_ext_label(self)
    }
}

#[derive(Debug, Clone)]
pub struct ShaderSource {
    pub source: String,
    pub name: Option<String>,
    pub kind: ShaderType,
}

impl ShaderModule for Program {
    type Gc = OpenGLContext;
    type Err = OpenGLError;
    type ShaderSource = ShaderSource;
    type Uniform = Uniform;
    type UniformLocation = u32;

    fn add_shader_source(&self, source: Self::ShaderSource) -> Result<(), Self::Err> {
        let shader = Shader::with_source(&self.gl, source.kind, &source.source)?;
        if let Some(name) = &source.name {
            shader.set_name(name);
        }
        unsafe { self.gl.AttachShader(self.id.get(), shader.id.get()) }
        self.shaders.insert(shader);
        OpenGLError::guard(&self.gl)
    }

    fn link(&self) -> Result<(), Self::Err> {
        let status = unsafe {
            self.gl.LinkProgram(self.id.get());
            let mut status = 0;
            self.gl
                .GetProgramiv(self.id.get(), gl::LINK_STATUS, &mut status);
            status as gl::types::GLboolean == gl::TRUE
        };
        if !status {
            let info_log = unsafe {
                let mut buf = vec![0; 2048];
                let mut len = 0;
                self.gl
                    .GetProgramInfoLog(self.id.get(), 2048, &mut len, buf.as_mut_ptr().cast());
                CStr::from_bytes_with_nul(&buf[..len as _])
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            };
            return Err(OpenGLError::with_info_log(&self.gl, info_log));
        }

        for shader in self.shaders.iter() {
            unsafe {
                self.gl.DetachShader(self.id.get(), shader.id.get());
            }
        }
        // Equivalent to drain, but doesn't return an iterator over the values
        self.shaders.retain(|_| false);
        Ok(())
    }

    fn uniform_location(&self, name: &str) -> Option<Self::UniformLocation> {
        unsafe {
            let loc = 0;
            let name = CString::new(name).unwrap();
            self.gl
                .GetUniformLocation(self.id.get(), name.as_c_str().as_ptr());
            (loc >= 0).then_some(loc as _)
        }
    }

    fn set_uniform(&self, location: Self::UniformLocation, uniform: impl AsUniform<Self>) {
        let location = location as _;
        unsafe {
            match uniform.into() {
                Uniform::Int(i) => self.gl.Uniform1i(location, i),
                Uniform::Uint(i) => self.gl.Uniform1ui(location, i),
                Uniform::Float(f) => self.gl.Uniform1f(location, f),
                Uniform::Ivec2(v) => self.gl.Uniform2iv(location, 1, v.as_ptr()),
                Uniform::UIvec2(v) => self.gl.Uniform2uiv(location, 1, v.as_ptr()),
                Uniform::Vec2(v) => self.gl.Uniform2fv(location, 1, v.as_ptr()),
                Uniform::Ivec3(v) => self.gl.Uniform3iv(location, 1, v.as_ptr()),
                Uniform::UIvec3(v) => self.gl.Uniform3uiv(location, 1, v.as_ptr()),
                Uniform::Vec3(v) => self.gl.Uniform3fv(location, 1, v.as_ptr()),
                Uniform::Ivec4(v) => self.gl.Uniform4iv(location, 1, v.as_ptr()),
                Uniform::UIvec4(v) => self.gl.Uniform4uiv(location, 1, v.as_ptr()),
                Uniform::Vec4(m) => self.gl.Uniform4fv(location, 1, m.as_ptr()),
                Uniform::Mat2(m) => {
                    self.gl
                        .UniformMatrix2fv(location, 1, gl::FALSE, m.as_ptr().cast())
                }
                Uniform::Mat3(m) => {
                    self.gl
                        .UniformMatrix3fv(location, 1, gl::FALSE, m.as_ptr().cast())
                }
                Uniform::Mat4(m) => {
                    self.gl
                        .UniformMatrix4fv(location, 1, gl::FALSE, m.as_ptr().cast())
                }
                Uniform::Mat23(m) => {
                    self.gl
                        .UniformMatrix2x3fv(location, 1, gl::FALSE, m.as_ptr().cast())
                }
                Uniform::Mat24(m) => {
                    self.gl
                        .UniformMatrix2x4fv(location, 1, gl::FALSE, m.as_ptr().cast())
                }
                Uniform::Mat32(m) => {
                    self.gl
                        .UniformMatrix3x2fv(location, 1, gl::FALSE, m.as_ptr().cast())
                }
                Uniform::Mat34(m) => {
                    self.gl
                        .UniformMatrix3x4fv(location, 1, gl::FALSE, m.as_ptr().cast())
                }
                Uniform::Mat42(m) => {
                    self.gl
                        .UniformMatrix4x2fv(location, 1, gl::FALSE, m.as_ptr().cast())
                }
                Uniform::Mat43(m) => {
                    self.gl
                        .UniformMatrix4x3fv(location, 1, gl::FALSE, m.as_ptr().cast())
                }
                _ => todo!(),
            }
        }
    }
}
