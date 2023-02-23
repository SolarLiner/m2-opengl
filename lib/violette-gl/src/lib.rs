use std::{
    ffi::{c_void, CString},
    sync::Arc,
};

use gl::types::GLenum;
use thread_guard::ThreadGuard;

pub mod api;
pub mod arrays;
pub mod buffer;
pub mod context;
pub mod debug;
pub mod framebuffer;
pub mod program;
mod thread_guard;
pub mod window;

type Gl = Arc<ThreadGuard<gl::Gl>>;

fn load_with(loader: impl FnMut(&'static str) -> *const c_void) -> Gl {
    let gl = gl::Gl::load_with(loader);
    Arc::new(ThreadGuard::new(gl))
}

trait GlObject {
    const GL_NAME: GLenum;
    fn gl(&self) -> &Gl;
    fn id(&self) -> u32;
}

// TODO: Figure out why glLabelObject does not work

fn get_ext_label<T>(_obj: &T) -> Option<String> {None}
#[cfg(never)]
fn get_ext_label<T: GlObject>(obj: &T) -> Option<String> {
    if obj.gl().GetObjectLabelEXT.is_loaded() {
        let name_result = unsafe {
            let mut len = 0;
            let mut data = vec![0u8; 2048];
            obj.gl().GetObjectLabelEXT(
                T::GL_NAME,
                obj.id(),
                2048,
                &mut len,
                data.as_mut_ptr().cast(),
            );
            CString::new(&data[..len as _])
        };
        match name_result {
            Ok(name) => Some(name.to_string_lossy().to_string()),
            Err(..) => {
                tracing::warn!("Could not fetch label from OpenGL: NUL byte found");
                None
            }
        }
    } else {
        None
    }
}

fn set_ext_label<T>(_obj: &T, _name: impl ToString) {}
#[cfg(never)]
fn set_ext_label<T: GlObject>(obj: &T, name: impl ToString) {
    if obj.gl().LabelObjectEXT.is_loaded() {
        let name_str = CString::new(name.to_string()).unwrap();
        unsafe {
            obj.gl().LabelObjectEXT(
                T::GL_NAME,
                obj.id(),
                name_str.as_bytes().len() as _,
                name_str.as_ptr(),
            );
        }
    }
}
