use std::{cell::RefCell, rc::Rc, sync::Arc, collections::HashSet, ffi::{CStr, CString}, marker::PhantomData, mem::ManuallyDrop, num::NonZeroU32, ops, sync::Mutex, fmt};
use std::borrow::BorrowMut;
use std::fmt::Formatter;

use cgmath::{Vector2, Vector3, Vector4};
use crevice::std140::*;
use dashmap::DashSet;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use violette_api::value::ValueType;
use violette_api::{
    bind::Bind,
    shader::{AsUniform, ShaderModule},
};
use violette_api::base::Resource;

use crate::{api::GlErrorKind, thread_guard::ThreadGuard, api::OpenGLError, context::OpenGLContext, Gl, get_ext_label, set_ext_label};

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
                NonZeroU32::new(gl::CreateProgram()).ok_or_else(|| {
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
        // unsafe {
        //     gl::BindShader(self.id.get());
        // }
    }

    fn unbind(&self) {
        // unsafe {
        //     gl::BindShader(0);
        // }
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Shader {
    gl: Gl,
    id: NonZeroU32,
}

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
            gl::DeleteShader(self.id.get());
        }
    }
}

impl Shader {
    pub fn new(gl: &Gl, typ: ShaderType) -> Result<Self, OpenGLError> {
        Ok(Self {
            gl: gl.clone(),
            id: unsafe {
                NonZeroU32::new(gl::CreateShader(typ as _)).ok_or_else(|| {
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
            gl::ShaderSource(
                self.id.get(),
                1,
                &source.as_c_str().as_ptr(),
                lengths.as_ptr(),
            );
        }
    }

    pub fn compile(&self) -> Result<(), OpenGLError> {
        let status = unsafe {
            gl::CompileShader(self.id.get());
            let mut status = 0;
            gl::GetShaderiv(self.id.get(), gl::COMPILE_STATUS, &mut status);
            status as gl::types::GLboolean == gl::FALSE
        };
        if !status {
            let info_log = unsafe {
                let mut buf = vec![0; 2048];
                let mut len = 0;
                gl::GetShaderInfoLog(self.id.get(), 2048, &mut len, buf.as_mut_ptr().cast());
                CStr::from_bytes_with_nul(&buf[..(len + 1) as _])
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            };
            Err(OpenGLError::with_info_log(gl, info_log).unwrap())
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

impl FromPrimitive for Uniform {
    fn from_isize(n: isize) -> Option<Self> {
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

impl Resource for Program {
    fn set_name(&self, name: impl ToString) {
        set_ext_label(self, name)
    }

    fn get_name(&self) -> Option<String> {
        get_ext_label(self)
    }
}

impl ShaderModule for Program {
    type Gc = OpenGLContext;
    type Err = OpenGLError;
    type ShaderSource = Shader;
    type Uniform = Uniform;
    type UniformLocation = u32;

    fn add_shader_source(&self, source: Self::ShaderSource) -> Result<(), Self::Err> {
        unsafe { self.gl.AttachShader(self.id.get(), source.id.get()) }
        self.shaders.insert(source);
        OpenGLError::guard(&self.gl)
    }

    fn link(&self) -> Result<(), Self::Err> {
        let status = unsafe {
            self.gl.LinkProgram(self.id.get());
            let mut status = 0;
            self.gl.GetProgramiv(self.id.get(), gl::LINK_STATUS, &mut status);
            status as gl::types::GLboolean == gl::TRUE
        };
        if !status {
            let info_log = unsafe {
                let mut buf = vec![0; 2048];
                let mut len = 0;
                self.gl.GetProgramInfoLog(self.id.get(), 2048, &mut len, buf.as_mut_ptr().cast());
                CStr::from_bytes_with_nul(&buf[..len as _])
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            };
            return Err(OpenGLError::with_info_log(&self.gl, info_log).unwrap());
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
            let mut loc = 0;
            let name = CString::new(name).unwrap();
            self.gl.GetUniformLocation(self.id.get(), name.as_c_str().as_ptr());
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
                Uniform::Mat2(m) => self.gl.UniformMatrix2fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat3(m) => self.gl.UniformMatrix3fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat4(m) => self.gl.UniformMatrix4fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat23(m) => self.gl.UniformMatrix2x3fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat24(m) => self.gl.UniformMatrix2x4fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat32(m) => self.gl.UniformMatrix3x2fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat34(m) => self.gl.UniformMatrix3x4fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat42(m) => self.gl.UniformMatrix4x2fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat43(m) => self.gl.UniformMatrix4x3fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                _ => todo!(),
            }
        }
    }
}
