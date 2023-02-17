use std::{
    collections::HashMap,
    sync::{Arc, RwLock, Weak},
    time::{Duration, Instant},
};

use eyre::Result;
use glam::{UVec2, Vec3};
use rose_core::{
    camera::Camera,
    gbuffers::GeometryBuffers,
    light::{GpuLight, Light, LightBuffer},
    material::Material,
    postprocess::Postprocess,
    screen_draw::ScreenDraw,
    transform::{TransformExt, Transformed},
    utils::thread_guard::ThreadGuard,
};
use tracing::span::EnteredSpan;
use violette::framebuffer::{ClearBuffer, Framebuffer};

pub type Mesh = rose_core::mesh::Mesh<rose_core::material::Vertex>;

#[derive(Debug, Clone, Copy)]
pub struct PostprocessInterface {
    pub exposure: f32,
    pub bloom: BloomInterface,
}

#[derive(Debug, Clone, Copy)]
pub struct BloomInterface {
    pub size: f32,
    pub strength: f32,
}

#[derive(Debug)]
pub struct Renderer {
    camera: Camera,
    lights: LightBuffer,
    geom_pass: Arc<RwLock<GeometryBuffers>>,
    post_process: Postprocess,
    post_process_iface: PostprocessInterface,
    queued_materials: Vec<Weak<Material>>,
    queued_meshes: HashMap<usize, Vec<Transformed<Weak<Mesh>>>>,
    render_span: ThreadGuard<Option<EnteredSpan>>,
    debug_window_open: bool,
    begin_scene_at: Option<Instant>,
    last_scene_duration: Option<Duration>,
    last_render_duration: Option<Duration>,
}

impl Renderer {
    pub fn new(size: UVec2) -> Result<Self> {
        let mut camera = Camera::default();
        camera.projection.update(size.as_vec2());
        let lights = LightBuffer::new();
        let geom_pass = GeometryBuffers::new(size)?;
        let post_process = Postprocess::new(size)?;
        Ok(Self {
            camera,
            lights,
            geom_pass: Arc::new(RwLock::new(geom_pass)),
            post_process,
            post_process_iface: PostprocessInterface {
                exposure: 1.,
                bloom: BloomInterface {
                    size: 1e-3,
                    strength: 1e-2,
                },
            },
            queued_materials: vec![],
            queued_meshes: HashMap::default(),
            render_span: ThreadGuard::new(None),
            begin_scene_at: None,
            last_scene_duration: None,
            last_render_duration: None,
            debug_window_open: false,
        })
    }

    pub fn post_process_interface(&mut self) -> &mut PostprocessInterface {
        &mut self.post_process_iface
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    #[tracing::instrument]
    pub fn resize(&mut self, size: UVec2) -> Result<()> {
        Framebuffer::backbuffer().viewport(0, 0, size.x as _, size.y as _);
        self.geom_pass.write().unwrap().resize(size)?;
        self.post_process.resize(size)?;
        self.camera.projection.update(size.as_vec2());
        Ok(())
    }

    #[tracing::instrument(skip(new_lights))]
    pub fn add_lights(&mut self, new_lights: impl IntoIterator<Item = Light>) -> Result<()> {
        let lights = if !self.lights.is_empty() {
            let existing_lights = GpuLight::download_buffer(&self.lights)?;
            existing_lights.into_iter().map(|gl| gl.into()).collect()
        } else {
            new_lights.into_iter().collect::<Vec<_>>()
        };
        self.lights = GpuLight::create_buffer(lights)?;
        Ok(())
    }

    pub fn begin_render(&mut self) -> Result<()> {
        self.render_span
            .replace(tracing::debug_span!("render").entered());
        self.begin_scene_at.replace(Instant::now());
        let backbuffer = Framebuffer::backbuffer();
        backbuffer.clear_color(Vec3::ZERO.extend(1.).to_array())?;
        backbuffer.clear_depth(1.)?;
        backbuffer.do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH)?;

        self.post_process
            .set_exposure(self.post_process_iface.exposure)?;
        self.post_process
            .set_bloom_size(self.post_process_iface.bloom.size)?;
        self.post_process
            .set_bloom_strength(self.post_process_iface.bloom.strength)?;

        self.geom_pass
            .read()
            .unwrap()
            .framebuffer()
            .do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH)?;
        Ok(())
    }

    #[tracing::instrument]
    pub fn submit_mesh(&mut self, material: Weak<Material>, mesh: Transformed<Weak<Mesh>>) {
        let mesh_ptr = Weak::as_ptr(&mesh) as usize;
        let material_ptr = Weak::as_ptr(&material) as usize;
        tracing::debug!(message="Submitting mesh", %mesh_ptr, %material_ptr);
        let mat_ix = if let Some(ix) = self
            .queued_materials
            .iter()
            .position(|mat| mat.ptr_eq(&material))
        {
            ix
        } else {
            let ix = self.queued_materials.len();
            self.queued_materials.push(material);
            ix
        };

        self.queued_meshes
            .entry(mat_ix)
            .and_modify(|v| v.push(mesh.clone()))
            .or_insert_with(|| vec![mesh]);
    }

    #[tracing::instrument]
    pub fn flush(&mut self) -> Result<()> {
        let render_start = Instant::now();
        let geom_pass = self.geom_pass.read().unwrap();
        for (mat_ix, meshes) in self.queued_meshes.drain() {
            let Some(material) = self.queued_materials[mat_ix].upgrade() else {
                tracing::warn!("Dropped material value, cannot recover from weakref");
                continue;
            };
            let Some(mut meshes) = meshes.into_iter().map(|w| w.upgrade().map(|v| v.transformed(w.transform))).collect::<Option<Vec<_>>>() else {
                tracing::warn!("Dropped mesh object, cannot recover from weakref");
                continue;
            };

            geom_pass.draw_meshes(&self.camera, &material, &mut meshes)?;
        }

        geom_pass.draw_screen(self.post_process.framebuffer(), &self.camera, &self.lights)?;
        self.post_process.draw(&Framebuffer::backbuffer())?;
        self.last_render_duration.replace(render_start.elapsed());
        self.last_scene_duration
            .replace(self.begin_scene_at.take().unwrap().elapsed());
        self.render_span.take();
        Ok(())
    }

    #[cfg(feature = "debug-ui")]
    pub fn ui_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.toggle_value(&mut self.debug_window_open, "Debug menu");
    }

    #[cfg(feature = "debug-ui")]
    pub fn ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("render-stats")
            .frame(
                egui::Frame::none()
                    .inner_margin(egui::style::Margin::same(5.))
                    .stroke(egui::Stroke::NONE),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Scene processing: {:2.2} ms",
                        self.last_scene_duration
                            .map(|d| d.as_secs_f64() * 1e3)
                            .unwrap_or(0.)
                    ));
                    ui.separator();
                    ui.label(format!(
                        "Render time: {:2.2} ms",
                        self.last_render_duration
                            .map(|d| d.as_secs_f64() * 1e3)
                            .unwrap_or(0.)
                    ));
                });
            });
        egui::Window::new("Renderer debug")
            .resizable(true)
            .open(&mut self.debug_window_open)
            .show(ctx, |ui| {
                egui::Grid::new("debug_textures")
                    .num_columns(2)
                    .show(ui, |ui| {
                        make_texture_frame(ui, "Position", {
                            let geom_pass = self.geom_pass.clone();
                            move |frame| geom_pass.read().unwrap().debug_position(frame).unwrap()
                        });
                        make_texture_frame(ui, "Albedo", {
                            let geom_pass = self.geom_pass.clone();
                            move |frame| geom_pass.read().unwrap().debug_albedo(frame).unwrap()
                        });
                        ui.end_row();

                        make_texture_frame(ui, "Normal", {
                            let geom_pass = self.geom_pass.clone();
                            move |frame| geom_pass.read().unwrap().debug_normal(frame).unwrap()
                        });
                        make_texture_frame(ui, "Roughness / Metal", {
                            let geom_pass = self.geom_pass.clone();
                            move |frame| geom_pass.read().unwrap().debug_rough_metal(frame).unwrap()
                        });
                        ui.end_row();
                    })
            });
    }
}

#[cfg(feature = "debug-ui")]
fn make_texture_frame(
    ui: &mut egui::Ui,
    name: &str,
    draw: impl 'static + Fn(&Framebuffer) + Send + Sync,
) -> egui::Response {
    ui.group(|ui| {
        let label = ui.label(name);
        let (rect, response) = ui.allocate_at_least(
            egui::vec2(128., 128.),
            egui::Sense::focusable_noninteractive(),
        );
        response.labelled_by(label.id);
        let painter = ui.painter();
        painter.rect_filled(rect, 0., egui::Rgba::from_gray(0.));
        painter.add(egui::PaintCallback {
            rect,
            callback: Arc::new(rose_ui::painter::UiCallback::new(move |info, ui| {
                draw(ui.framebuffer());
            })),
        });
    })
    .response
}
