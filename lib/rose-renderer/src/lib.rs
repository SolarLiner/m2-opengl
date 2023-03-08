use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
    time::{Duration, Instant},
};

use eyre::Result;
use float_ord::FloatOrd;
use glam::{UVec2, vec2, Vec3, Vec4Swizzles};
use tracing::span::EnteredSpan;

use gbuffers::GeometryBuffers;
use material::Material;
use postprocess::Postprocess;
use rose_core::{
    camera::Camera,
    light::{GpuLight, Light, LightBuffer},
    transform::{Transformed, TransformExt},
    utils::thread_guard::ThreadGuard,
};
use rose_core::camera::{ViewUniform, ViewUniformBuffer};
use violette::{
    Cull,
    framebuffer::{ClearBuffer, DepthTestFunction, Framebuffer}, FrontFace,
};
use violette::base::resource::ResourceExt;

use crate::env::Environment;
use crate::material::MaterialInstance;
use crate::postprocess::LensFlareParams;

pub mod env;
pub mod gbuffers;
pub mod material;
pub mod postprocess;

pub type Mesh = rose_core::mesh::Mesh<material::Vertex>;

#[derive(Debug, Clone, Copy)]
pub struct PostprocessInterface {
    pub exposure: f32,
    pub bloom: BloomInterface,
    pub lens_flare: LensFlareParams,
}

impl PostprocessInterface {
    #[cfg(feature = "debug-ui")]
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        use egui::{
            DragValue,
            Grid,
        };

        ui.collapsing("Main", |ui| {
            Grid::new("pp-iface")
                .striped(true)
                .num_columns(2)
                .show(ui, |ui| {
                    let exposure_label = ui.label("Exposure:");
                    ui.add(
                        egui::Slider::new(&mut self.exposure, 1e-6..=1e4)
                            .logarithmic(true)
                            .show_value(true)
                            .suffix(" EV")
                            .custom_formatter(|v, _| format!("{:+1.1}", v.log2()))
                            .custom_parser(|s| s.parse().ok().map(|ev| 2f64.powf(ev)))
                            .text("Exposure"),
                    )
                        .labelled_by(exposure_label.id);
                    ui.end_row();

                    let bloom_size_label = ui.label("Bloom size:");
                    ui.add(
                        egui::Slider::new(&mut self.bloom.size, 0f32..=0.5)
                            .logarithmic(true)
                            .clamp_to_range(false)
                            .show_value(true),
                    )
                        .labelled_by(bloom_size_label.id);
                    ui.end_row();

                    let bloom_strength_label = ui.label("Bloom strength:");
                    ui.add(
                        egui::Slider::new(&mut self.bloom.strength, 1e-4..=1e2)
                            .logarithmic(true)
                            .show_value(true)
                            .suffix(" %")
                            .custom_formatter(|x, _| format!("{:2.1}", x * 100.))
                            .custom_parser(|s| s.parse().ok().map(|x: f64| x / 100.))
                            .text("Bloom strength"),
                    )
                        .labelled_by(bloom_strength_label.id);
                    ui.end_row();
                });
        });
        ui.collapsing("Lens Flare", |ui| {
            Grid::new("postprocess-lens-flares")
                .num_columns(2)
                .show(ui, |ui| {
                    let strength_label = ui.label("Strength").id;
                    self.lens_flare.strength *= 100.;
                    ui.add(DragValue::new(&mut self.lens_flare.strength).suffix(" %")).labelled_by(strength_label);
                    self.lens_flare.strength /= 100.;
                    ui.end_row();

                    let threshold_label = ui.label("Threshold").id;
                    self.lens_flare.threshold = self.lens_flare.threshold.log2();
                    ui.add(DragValue::new(&mut self.lens_flare.threshold).suffix(" EV")).labelled_by(threshold_label);
                    self.lens_flare.threshold = self.lens_flare.threshold.exp2();
                    ui.end_row();

                    let dist_label = ui.label("Distortion").id;
                    ui.add(DragValue::new(&mut self.lens_flare.distortion)).labelled_by(dist_label);
                    ui.end_row();

                    let ghost_spacing_label = ui.label("Ghost spacing").id;
                    ui.add(DragValue::new(&mut self.lens_flare.ghost_spacing).speed(0.01)).labelled_by(ghost_spacing_label);
                    ui.end_row();

                    let ghost_count_label = ui.label("Ghost count").id;
                    ui.add(DragValue::new(&mut self.lens_flare.ghost_count)).labelled_by(ghost_count_label);
                });
        });
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BloomInterface {
    pub size: f32,
    pub strength: f32,
}

#[derive(Debug)]
pub struct Renderer {
    lights: LightBuffer,
    geom_pass: Rc<RefCell<GeometryBuffers>>,
    material: Material,
    post_process: Postprocess,
    post_process_iface: PostprocessInterface,
    environment: Option<Box<dyn Environment>>,
    view_uniform: ViewUniform,
    camera_uniform: ThreadGuard<ViewUniformBuffer>,
    queued_materials: Vec<Weak<MaterialInstance>>,
    queued_meshes: HashMap<usize, Vec<Transformed<Weak<Mesh>>>>,
    render_span: ThreadGuard<Option<EnteredSpan>>,
    debug_window_open: bool,
    begin_scene_at: Option<Instant>,
    last_scene_duration: Option<Duration>,
    last_render_duration: Option<Duration>,
    last_render_submitted: usize,
    last_render_rendered: usize,
}

impl Renderer {}

impl Renderer {
    pub fn new(size: UVec2) -> Result<Self> {
        let lights = LightBuffer::new();
        let geom_pass = GeometryBuffers::new(size)?;
        let post_process = Postprocess::new(size)?;
        let view_uniform = ViewUniform::default();
        let camera_uniform = view_uniform.create_buffer()?;

        Ok(Self {
            lights,
            geom_pass: Rc::new(RefCell::new(geom_pass)),
            material: Material::create(Some(&camera_uniform))?,
            post_process,
            post_process_iface: PostprocessInterface {
                exposure: 1.5f32.exp2(),
                bloom: BloomInterface {
                    size: 1e-3,
                    strength: 4e-2,
                },
                lens_flare: LensFlareParams::default(),
            },
            environment: None,
            view_uniform,
            camera_uniform: ThreadGuard::new(camera_uniform),
            queued_materials: vec![],
            queued_meshes: HashMap::default(),
            render_span: ThreadGuard::new(None),
            begin_scene_at: None,
            last_scene_duration: None,
            last_render_duration: None,
            last_render_submitted: 0,
            last_render_rendered: 0,
            debug_window_open: false,
        })
    }

    pub fn post_process_interface(&mut self) -> &mut PostprocessInterface {
        &mut self.post_process_iface
    }

    #[tracing::instrument]
    pub fn resize(&mut self, size: UVec2) -> Result<()> {
        Framebuffer::viewport(0, 0, size.x as _, size.y as _);
        self.geom_pass.borrow_mut().resize(size)?;
        self.post_process.resize(size)?;
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

    pub fn set_environment<E: Environment>(&mut self, env: E) {
        self.environment.replace(Box::new(env));
    }

    pub fn environment<E: Environment>(&self) -> Option<&E> {
        self.environment
            .as_deref()
            .and_then(|b| b.as_any().downcast_ref())
    }

    pub fn environment_mut<E: Environment>(&mut self) -> Option<&mut E> {
        self.environment
            .as_deref_mut()
            .and_then(|b| b.as_any_mut().downcast_mut())
    }

    pub fn set_light_buffer(&mut self, light_buffer: LightBuffer) {
        self.lights = light_buffer;
    }

    pub fn begin_render(&mut self, camera: &Camera) -> Result<()> {
        self.render_span
            .replace(tracing::debug_span!("render").entered());
        let now = Instant::now();
        tracing::trace!(message = "Begin render", ?now);
        self.begin_scene_at.replace(now);

        self.last_render_rendered = 0;
        self.last_render_submitted = 0;

        self.post_process.luminance_bias = self.post_process_iface.exposure;
        self.post_process.bloom_radius = self.post_process_iface.bloom.size;
        self.post_process
            .set_bloom_strength(self.post_process_iface.bloom.strength)?;
        self.post_process.set_lens_flare_parameters(self.post_process_iface.lens_flare)?;

        self.view_uniform.update_from_camera(camera);
        self.view_uniform
            .update_uniform_buffer(&mut self.camera_uniform)?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub fn submit_mesh(&mut self, material: Weak<MaterialInstance>, mesh: Transformed<Weak<Mesh>>) {
        let mesh_ptr = Weak::as_ptr(&mesh) as usize;
        let material_ptr = Weak::as_ptr(&material) as usize;
        self.last_render_submitted += 1;
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

    #[tracing::instrument(skip(self))]
    pub fn flush(&mut self, dt: Duration, clear_color: Vec3) -> Result<()> {
        let render_start = Instant::now();
        violette::set_front_face(FrontFace::CounterClockwise);
        violette::culling(Some(Cull::Back));
        let [w, h] = self.view_uniform.viewport.zw().as_ivec2().to_array();
        Framebuffer::viewport(0, 0, w, h);
        Framebuffer::enable_depth_test(DepthTestFunction::Less);
        Framebuffer::disable_scissor();
        Framebuffer::disable_blending();
        Framebuffer::clear_color([0., 0., 0., 0.]);

        self.geom_pass
            .borrow()
            .framebuffer()
            .do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH);

        let geom_pass = self.geom_pass.borrow();
        self.material.set_camera_uniform(&self.camera_uniform)?;
        for (mat_ix, meshes) in self.queued_meshes.drain() {
            let Some(instance) = self.queued_materials[mat_ix].upgrade() else {
                tracing::warn!("Dropped materials value, cannot recover from weakref");
                continue;
            };
            let Some(mut meshes) = meshes.into_iter().map(|w| w.upgrade().map(|v| v.transformed(w.transform))).collect::<Option<Vec<_>>>() else {
                tracing::warn!("Dropped mesh object, cannot recover from weakref");
                continue;
            };

            meshes.sort_by_cached_key(|m| {
                FloatOrd(
                    m.transform
                        .position
                        .distance_squared(self.view_uniform.camera_pos),
                )
            });

            self.last_render_rendered += meshes.len();
            geom_pass.draw_meshes(&self.material, instance.as_ref(), &meshes)?;
        }

        Framebuffer::disable_depth_test();
        Framebuffer::clear_color(clear_color.extend(1.).to_array());
        let backbuffer = Framebuffer::backbuffer();
        backbuffer.do_clear(ClearBuffer::COLOR);
        let shaded_tex = geom_pass.process(
            &self.camera_uniform,
            &self.lights,
            self.environment.as_deref_mut(),
        )?;
        Framebuffer::disable_blending();
        self.post_process.draw(&backbuffer, shaded_tex, dt)?;
        self.last_render_duration.replace(render_start.elapsed());
        self.last_scene_duration
            .replace(self.begin_scene_at.take().unwrap().elapsed());
        self.render_span.take();
        Ok(())
    }

    #[cfg(feature = "debug-ui")]
    pub fn ui_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.toggle_value(&mut self.debug_window_open, "Debug menu");
        ui.menu_button("Post processing", |ui| {
            let pp_iface = self.post_process_interface();
            pp_iface.ui(ui);
        });
    }

    #[cfg(feature = "debug-ui")]
    pub fn ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("renderer-stats").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.ui_render_stats(ui);
            });
        });
        // egui::Window::new("Renderer debug")
        //     .resizable(true)
        //     .open(&mut self.debug_window_open)
        // .show(ctx, |ui| {
        //     self.ui_debug_panel(ui);
        // });
    }

    #[cfg(feature = "debug-ui")]
    pub fn ui_render_stats(&mut self, ui: &mut egui::Ui) {
        ui.label(format!(
            "{:3} Objects submitted | {:3} objects rendered",
            self.last_render_submitted, self.last_render_rendered
        ));
        ui.separator();
        ui.label(format!(
            "Scene processing: {:5?}",
            self.last_scene_duration.unwrap_or_default()
        ));
        ui.separator();
        ui.label(format!(
            "Render time: {:5?}",
            self.last_render_duration.unwrap_or_default()
        ));
        ui.separator();
        ui.label(format!(
            "Average luminance: {:>2.2} EV",
            self.post_process.average_luminance().log2()
        ));
    }

    #[cfg(feature = "debug-ui")]
    pub fn ui_debug_panel(&self, ui: &mut egui::Ui) {
        const GET_NAME: fn(usize) -> &'static str = |ix| match ix {
            0 => "Position",
            1 => "Albedo",
            2 => "Normal",
            3 => "Roughness/Metal",
            _ => "<None>",
        };
        thread_local! {
            static SELECTED_TEXTURE: RefCell<usize> = RefCell::new(usize::MAX);
        }
        SELECTED_TEXTURE.with(|key| {
            let ix = ui
                .horizontal(|ui| {
                    let label = ui.label("Select debug texture");
                    let ix = &mut *key.borrow_mut();
                    egui::ComboBox::new("renderer-debug-texture", "Debug texture")
                        .selected_text(GET_NAME(*ix))
                        .show_index(ui, ix, 5, |ix| GET_NAME(ix).to_string())
                        .labelled_by(label.id);
                    *ix
                })
                .inner;
            const SIDE: f32 = 256.;
            let size = self.geom_pass.borrow().size().as_vec2();
            let size = if size.x > size.y {
                vec2(SIDE, size.y / size.x * SIDE)
            } else {
                vec2(SIDE * size.x / size.y, SIDE)
            };
            let (rect, _) = ui.allocate_at_least(
                egui::vec2(size.x, size.y),
                egui::Sense::focusable_noninteractive(),
            );
            let painter = ui.painter();
            painter.rect_filled(rect, 0., egui::Rgba::from_gray(0.));
            let geom_pass = self.geom_pass.clone();
            painter.add(egui::PaintCallback {
                rect,
                callback: Arc::new(rose_ui::painter::UiCallback::new(move |_info, ui| {
                    let geom_pass = geom_pass.borrow();
                    let _ = match ix {
                        0 => geom_pass.debug_position(ui.framebuffer()),
                        1 => geom_pass.debug_albedo(ui.framebuffer()),
                        2 => geom_pass.debug_normal(ui.framebuffer()),
                        3 => geom_pass.debug_rough_metal(ui.framebuffer()),
                        _ => Ok(()),
                    }
                        .is_ok();
                })),
            });
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
            callback: Arc::new(rose_ui::painter::UiCallback::new(move |_info, ui| {
                draw(ui.framebuffer());
            })),
        });
    })
    .response
}
