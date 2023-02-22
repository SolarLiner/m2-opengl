use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::{
    collections::HashSet,
    ffi::{CStr, CString},
    marker::PhantomData,
    mem::ManuallyDrop,
    num::NonZeroU32,
    ops,
    sync::Mutex,
};

use cgmath::{Vector2, Vector3, Vector4};
use crevice::std140::*;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use violette_api::value::ValueType;
use violette_api::{
    bind::Bind,
    shader::{AsUniform, ShaderModule},
};

use crate::api::GlErrorKind;
use crate::thread_guard::ThreadGuard;
use crate::{api::OpenGLError, context::OpenGLContext};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, FromPrimitive)]
#[repr(u32)]
pub enum ShaderType {
    Vertex = gl::VERTEX_SHADER,
    Fragment = gl::FRAGMENT_SHADER,
    Geometry = gl::GEOMETRY_SHADER,
}
#[derive(Debug)]
pub struct ProgramImpl {
    __non_send: PhantomData<*mut ()>,
    id: NonZeroU32,
    shaders: RefCell<HashSet<Shader>>,
}

impl Drop for ProgramImpl {
    fn drop(&mut self) {
        unsafe { gl::DeleteShader(self.id.get()) }
    }
}

#[derive(Debug)]
pub struct Program(ThreadGuard<ProgramImpl>);

impl ops::Deref for Program {
    type Target = ProgramImpl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Program {
    pub fn new() -> Result<Self, OpenGLError> {
        let inner = ProgramImpl {
            __non_send: PhantomData,
            id: unsafe {
                NonZeroU32::new(gl::CreateProgram()).ok_or_else(|| {
                    OpenGLError::from(
                        GlErrorKind::current_error().unwrap_or(GlErrorKind::UnknownError),
                    )
                })?
            },
            shaders: RefCell::new(HashSet::new()),
        };
        Ok(Self(ThreadGuard::new(inner)))
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
    __non_send: PhantomData<*mut ()>,
    id: NonZeroU32,
}

impl Shader {
    pub fn with_source(typ: ShaderType, source: &str) -> Result<Self, OpenGLError> {
        let this = Self::new(typ)?;
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
    pub fn new(typ: ShaderType) -> Result<Self, OpenGLError> {
        Ok(Self {
            __non_send: PhantomData,
            id: unsafe {
                NonZeroU32::new(gl::CreateShader(typ as _)).ok_or_else(|| {
                    OpenGLError::from(
                        GlErrorKind::current_error().unwrap_or(GlErrorKind::UnknownError),
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
            Err(OpenGLError::with_info_log(info_log).unwrap())
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

impl ShaderModule for Program {
    type Gc = OpenGLContext;
    type Err = OpenGLError;
    type ShaderSource = Shader;
    type Uniform = Uniform;
    type UniformLocation = u32;

    fn add_shader_source(&self, source: Self::ShaderSource) -> Result<(), Self::Err> {
        unsafe { gl::AttachShader(self.id.get(), source.id.get()) }
        self.shaders.borrow_mut().insert(source);
        OpenGLError::guard()
    }

    fn link(&self) -> Result<(), Self::Err> {
        let status = unsafe {
            gl::LinkProgram(self.id.get());
            let mut status = 0;
            gl::GetProgramiv(self.id.get(), gl::LINK_STATUS, &mut status);
            status as gl::types::GLboolean == gl::TRUE
        };
        if !status {
            let info_log = unsafe {
                let mut buf = vec![0; 2048];
                let mut len = 0;
                gl::GetProgramInfoLog(self.id.get(), 2048, &mut len, buf.as_mut_ptr().cast());
                CStr::from_bytes_with_nul(&buf[..len as _])
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            };
            return Err(OpenGLError::with_info_log(info_log).unwrap());
        }

        for shader in self.shaders.borrow_mut().drain() {
            unsafe {
                gl::DetachShader(self.id.get(), shader.id.get());
            }
        }
        Ok(())
    }

    fn uniform_location(&self, name: &str) -> Option<Self::UniformLocation> {
        unsafe {
            let mut loc = 0;
            let name = CString::new(name).unwrap();
            gl::GetUniformLocation(self.id.get(), name.as_c_str().as_ptr());
            (loc >= 0).then_some(loc as _)
        }
    }

    fn set_uniform(&self, location: Self::UniformLocation, uniform: impl AsUniform<Self>) {
        let location = location as _;
        unsafe {
            match uniform.into() {
                Uniform::Int(i) => gl::Uniform1i(location, i),
                Uniform::Uint(i) => gl::Uniform1ui(location, i),
                Uniform::Float(f) => gl::Uniform1f(location, f),
                Uniform::Ivec2(v) => gl::Uniform2iv(location, 1, v.as_ptr()),
                Uniform::UIvec2(v) => gl::Uniform2uiv(location, 1, v.as_ptr()),
                Uniform::Vec2(v) => gl::Uniform2fv(location, 1, v.as_ptr()),
                Uniform::Ivec3(v) => gl::Uniform3iv(location, 1, v.as_ptr()),
                Uniform::UIvec3(v) => gl::Uniform3uiv(location, 1, v.as_ptr()),
                Uniform::Vec3(v) => gl::Uniform3fv(location, 1, v.as_ptr()),
                Uniform::Ivec4(v) => gl::Uniform4iv(location, 1, v.as_ptr()),
                Uniform::UIvec4(v) => gl::Uniform4uiv(location, 1, v.as_ptr()),
                Uniform::Vec4(m) => gl::Uniform4fv(location, 1, m.as_ptr()),
                Uniform::Mat2(m) => gl::UniformMatrix2fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat3(m) => gl::UniformMatrix3fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat4(m) => gl::UniformMatrix4fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat23(m) => gl::UniformMatrix2x3fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat24(m) => gl::UniformMatrix2x4fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat32(m) => gl::UniformMatrix3x2fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat34(m) => gl::UniformMatrix3x4fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat42(m) => gl::UniformMatrix4x2fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                Uniform::Mat43(m) => gl::UniformMatrix4x3fv(location, 1, gl::FALSE, m.as_ptr().cast()),
                _ => todo!(),
            }
        }
    }
}
