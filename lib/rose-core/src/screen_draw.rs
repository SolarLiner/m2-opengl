use std::cell::{Ref, RefCell};
use std::path::Path;

use eyre::{Context, Result};
use once_cell::sync::Lazy;

use violette::framebuffer::Framebuffer;
use violette::shader::{FragmentShader, VertexShader};
use violette::{
    buffer::{Buffer, ElementBuffer},
    program::Program,
    vertex::{DrawMode, VertexArray},
};

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
    pub fn new<'s>(
        shader_sources: impl IntoIterator<Item = &'s str>,
        reload_watcher: ReloadFileProxy,
    ) -> Result<Self> {
        let vert_shader = VertexShader::new(SCREEN_VS)?;
        let frag_shader = FragmentShader::new_multiple(shader_sources)?;
        let program = Program::new()
            .with_shader(vert_shader.id)
            .with_shader(frag_shader.id)
            .link()?;
        program.validate()?;
        Ok(Self {
            program: RefCell::new(program),
            reload_watcher,
        })
    }

    pub fn load(file: impl AsRef<Path>, reload_watcher: &ReloadWatcher) -> Result<Self> {
        let filepath = reload_watcher.base_path().join(&file);
        let files = glsl_preprocessor::load_and_parse(&filepath)
            .with_context(|| format!("Loading {}", file.as_ref().display()))?;
        Self::new(
            files.iter().map(|(_, s)| s.as_str()),
            reload_watcher.proxy(files.iter().map(|(p, _)| p.as_path())),
        )
        .with_context(|| format!("Loading shader {}", filepath.display()))
        .with_context(|| {
            format!(
                "File map:\n{}",
                files
                    .iter()
                    .map(|(p, _)| p.as_path())
                    .enumerate()
                    .map(|(ix, p)| format!("\t{} => {}", ix, p.display()))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
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
                        let files = glsl_preprocessor::load_and_parse(frag_path)?;
                        tracing::info!(message="Reloading screen-space shader", path=%frag_path.display());
                        let new_program_result = (|| {
                            let vs = VertexShader::new(SCREEN_VS)?;
                            let fs = FragmentShader::new_multiple(
                                files.iter().map(|(_, s)| s.as_str()),
                            )?;
                            let program = Program::new()
                                .with_shader(vs.id)
                                .with_shader(fs.id)
                                .link()?;
                            program.validate()?;
                            Ok::<_, eyre::Report>(program)
                        })();
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
