use std::{ops, thread::ThreadId};

/// Guard a non-send value by recording its originating thread.
///
/// This wrapper allows to bypass the Send restriction on types, with the caveat that the guard
/// will only succeed if accessed in the original thread. This allows "sending" the value as an opaque
/// object, while still meeting Rust's memory safety requirements.
///
/// The specific use case is to allow Non-Send OpenGL resources to be sent across thread to the renderer,
/// allowing it to work in a multi-threaded environment, while still only allowing their use on the OpenGL
/// thread.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ThreadGuard<T> {
    value: T,
    thread_id: ThreadId,
}

// # Safety
// All accesses to the inner value are guarded by checking that the thread accessing it is the right one.
// In effect, this allows sending the value, but disallows using it anywhere but in the right thread.
unsafe impl<T> Send for ThreadGuard<T> {}
unsafe impl<T> Sync for ThreadGuard<T> {}

impl<T> ThreadGuard<T> {
    pub fn new(value: T) -> Self {
        let thread_id = std::thread::current().id();
        Self { thread_id, value }
    }

    pub fn is_current_thread(&self) -> bool {
        self.thread_id == std::thread::current().id()
    }

    pub fn try_into_inner(self) -> Result<T, Self> {
        if self.is_current_thread() {
            Ok(self.value)
        } else {
            Err(self)
        }
    }

    #[inline(always)]
    fn assert_current_thread_ok(&self) {
        if !self.is_current_thread() {
            panic!("Tried to access value from the wrong thread");
        }
    }

    pub fn get(&self) -> Option<&T> {
        self.is_current_thread().then_some(&self.value)
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.is_current_thread().then_some(&mut self.value)
    }
}

impl<T> ops::Deref for ThreadGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.assert_current_thread_ok();
        &self.value
    }
}

impl<T> ops::DerefMut for ThreadGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.assert_current_thread_ok();
        &mut self.value
    }
}
