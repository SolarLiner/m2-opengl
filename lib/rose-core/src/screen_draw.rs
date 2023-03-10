use std::cell::{Ref, RefCell};
use std::path::Path;

use eyre::{Context, Result};
use once_cell::sync::Lazy;

use violette::{
    buffer::{Buffer, ElementBuffer},
    program::Program,
    vertex::{DrawMode, VertexArray},
};
use violette::framebuffer::Framebuffer;

use crate::utils::{
    reload_watcher::{ReloadFileProxy, ReloadWatcher},
    thread_guard::ThreadGuard,
};

const INDICES: [u32; 6] = [/* Face 1: */ 0, 2, 1, /* Face 2: */ 0, 3, 2];
static SCREEN_INDEX_BUFFER: Lazy<ThreadGuard<ElementBuffer<u32>>> =
    Lazy::new(|| ThreadGuard::new(Buffer::with_data(&INDICES).unwrap()));
static SCREEN_VAO: Lazy<ThreadGuard<VertexArray>> = Lazy::new(|| {
    let mut vao = VertexArray::new();
    vao.with_element_buffer(&*SCREEN_INDEX_BUFFER).unwrap();
    ThreadGuard::new(vao)
});

#[derive(Debug)]
pub struct ScreenDraw {
    program: RefCell<Program>,
    reload_watcher: ReloadFileProxy,
}

impl ScreenDraw {
    pub fn new(shader_source: &str, reload_watcher: ReloadFileProxy) -> Result<Self> {
        let program = Program::from_sources(SCREEN_VS, Some(shader_source), None)
            .context("Could not compile OpenGL shader program")?;
        Ok(Self {
            program: RefCell::new(program),
            reload_watcher,
        })
    }

    pub fn load(file: impl AsRef<Path>, reload_watcher: &ReloadWatcher) -> Result<Self> {
        let file = file.as_ref();
        let filepath = reload_watcher.base_path().join(file);
        Self::new(
            std::fs::read_to_string(&filepath)
                .context(format!(
                    "Cannot read shader from file {}",
                    filepath.display()
                ))?
                .as_str(),
            reload_watcher.proxy_single(filepath.as_path()),
        )
            .with_context(|| format!("Loading shader {}", filepath.display()))
    }

    pub fn program(&self) -> Ref<Program> {
        self.program.borrow()
    }

    #[tracing::instrument(skip_all)]
    pub fn draw(&self, framebuffer: &Framebuffer) -> Result<()> {
        match self.program.try_borrow_mut() {
            Ok(mut program) => {
                if self.reload_watcher.should_reload() {
                    let mut paths = self.reload_watcher.paths();
                    if let Some(frag_path) = paths.next() {
                        tracing::info!(message="Reloading screen-space shader", path=%frag_path.display());
                        let data = std::fs::read_to_string(frag_path).unwrap();
                        let new_program_result =
                            Program::from_sources(SCREEN_VS, Some(&*data), None);
                        match new_program_result {
                            Ok(new_program) => {
                                let _ = std::mem::replace(&mut *program, new_program);
                            }
                            Err(err) => {
                                tracing::warn!("Cannot reload shader: {}", err);
                            }
                        }
                    }
                }
            }
            Err(err) => {
                tracing::warn!("Cannot update program: {}", err);
            }
        }
        Framebuffer::disable_depth_test();
        framebuffer.draw_elements(
            &self.program.borrow(),
            &SCREEN_VAO,
            DrawMode::Triangles,
            0..6,
        )?;
        Ok(())
    }
}

const SCREEN_VS: &str = include_str!("./screen.vert.glsl");
