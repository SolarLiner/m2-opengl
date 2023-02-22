use std::{cell::RefCell, ffi::c_void};

use gl::types::{GLchar, GLenum, GLsizei, GLuint};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u32)]
pub enum CallbackSource {
    Api = gl::DEBUG_SOURCE_API,
    WindowSystem = gl::DEBUG_SOURCE_WINDOW_SYSTEM,
    ShaderCompiler = gl::DEBUG_SOURCE_SHADER_COMPILER,
    ThirdParty = gl::DEBUG_SOURCE_THIRD_PARTY,
    Application = gl::DEBUG_SOURCE_APPLICATION,
    Other = gl::DEBUG_SOURCE_OTHER,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u32)]
pub enum CallbackType {
    Error = gl::DEBUG_TYPE_ERROR,
    DeprecatedBehavior = gl::DEBUG_TYPE_DEPRECATED_BEHAVIOR,
    UndefinedBehavior = gl::DEBUG_TYPE_UNDEFINED_BEHAVIOR,
    TypePortability = gl::DEBUG_TYPE_PORTABILITY,
    TypePerformance = gl::DEBUG_TYPE_PERFORMANCE,
    TypeMarker = gl::DEBUG_TYPE_MARKER,
    PushGroup = gl::DEBUG_TYPE_PUSH_GROUP,
    PopGroup = gl::DEBUG_TYPE_POP_GROUP,
    Other = gl::DEBUG_TYPE_OTHER,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
#[repr(u32)]
pub enum CallbackSeverity {
    High = gl::DEBUG_SEVERITY_HIGH,
    Medium = gl::DEBUG_SEVERITY_MEDIUM,
    Low = gl::DEBUG_SEVERITY_LOW,
    Notification = gl::DEBUG_SEVERITY_NOTIFICATION,
}

#[derive(Debug, Clone)]
pub struct GlDebugData {
    pub source: CallbackSource,
    pub r#type: CallbackType,
    pub message: String,
    pub id: u32,
    pub severity: CallbackSeverity,
}

type UserCallback = Box<dyn Fn(GlDebugData)>;

static mut USER_CALLBACK: RefCell<Option<UserCallback>> = RefCell::new(None);

extern "system" fn message_callback(
    source: GLenum,
    r#type: GLenum,
    id: GLuint,
    severity: GLenum,
    length: GLsizei,
    message: *const GLchar,
    _user_param: *mut c_void,
) {
    if let Some(user_callback) = unsafe { USER_CALLBACK.get_mut().as_mut() } {
        let data = GlDebugData {
            source: CallbackSource::from_u32(source).unwrap(),
            r#type: CallbackType::from_u32(r#type).unwrap(),
            message: {
                let buf = bytemuck::cast_slice(unsafe {
                    std::slice::from_raw_parts(message, length as _)
                });
                String::from_utf8_lossy(buf).to_string()
            },
            id,
            severity: CallbackSeverity::from_u32(severity).unwrap(),
        };
        user_callback(data);
    }
}

pub fn set_message_callback<F: 'static + Fn(GlDebugData)>(cb: F) {
    if !gl::DebugMessageCallback::is_loaded() {
        tracing::warn!("glDebugMessageCallback is not available, cannot set debug callback");
    } else {
        unsafe {
            USER_CALLBACK.get_mut().replace(Box::new(cb));
            gl::Enable(gl::DEBUG_OUTPUT);
            gl::DebugMessageCallback(Some(message_callback), std::ptr::null_mut());
        }
    }
}
