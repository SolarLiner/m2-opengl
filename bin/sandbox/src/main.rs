use std::{
    cell::RefCell,
    path::PathBuf,
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    sync::Mutex,
    sync::{Arc, RwLock, Weak},
    thread::JoinHandle,
};

use egui::{emath::Numeric, Response, RichText, Ui, Widget};
use egui_extras::Column;
use egui_gizmo::{Gizmo, GizmoMode};
use eyre::Result;
use glam::{vec2, vec3, Mat4, UVec2, Vec2, Vec3};
use image::DynamicImage;
use tracing::Instrument;

use pan_orbit_camera::{OrbitCameraController, OrbitCameraInteractionController};
use rose_core::{
    light::Light,
    material::{Material, TextureSlot, Vertex},
    mesh::MeshBuilder,
    transform::{Transform, TransformExt, Transformed},
};
use rose_platform::{
    events::WindowEvent, Application, LogicalSize, PhysicalSize, RenderContext, TickContext,
    UiContext, WindowBuilder,
};
use rose_renderer::{Mesh, Renderer};
use violette::texture::{AsTextureFormat, Texture};

use crate::{
    read_mesh::ObjectData,
    scene::{Entity, Scene},
};

mod read_mesh;
mod scene;
// mod persistence;

type Respond<T> =
    Option<Box<dyn Send + Sync + FnOnce(T, &mut Vec<UiMessage>, &mut Vec<RenderMessage>)>>;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum NewLightType {
    Ambient,
    Directional,
    Point,
}

#[derive(Debug, Copy, Clone)]
struct NewLight {
    ty: NewLightType,
    transform: Transform,
    color: [f32; 3],
}

enum TextureDesc<const N: usize> {
    Image(DynamicImage),
    Color([f32; N]),
}

enum UiMessage {
    OpenMesh,
    LoadMesh {
        filepath: PathBuf,
        respond: Respond<Box<dyn Send + Sync + ObjectData>>,
    },
    AddLight {
        light: Light,
        respond: Respond<Weak<Light>>,
    },
    InstanceLight {
        light: Transformed<Weak<Light>>,
        respond: Respond<u64>,
    },
    InstanceObject {
        mesh: Transformed<Weak<Mesh>>,
        material: Weak<Material>,
        respond: Respond<u64>,
    },
    Select(u64),
    Deselect,
    DeleteInstance(u64),
}

enum RenderMessage {
    AddSphere {
        radius: f32,
        nlat: usize,
        nlon: usize,
        respond: Respond<Weak<Mesh>>,
    },
    CreateMaterial {
        albedo: TextureDesc<3>,
        normal: Option<DynamicImage>,
        rough_metal: TextureDesc<2>,
        respond: Respond<Weak<Material>>,
    },
    LoadMesh(Box<dyn Send + Sync + ObjectData>),
}

#[derive(Debug)]
struct Combined<T> {
    rx: Arc<Mutex<Receiver<T>>>,
    tx: Arc<Mutex<SyncSender<T>>>,
}

impl<T> Clone for Combined<T> {
    fn clone(&self) -> Self {
        Self {
            rx: self.rx.clone(),
            tx: self.tx.clone(),
        }
    }
}

impl<T> Combined<T> {
    pub fn new() -> Self {
        let (tx, rx) = sync_channel(16);
        Self {
            tx: Arc::new(Mutex::new(tx)),
            rx: Arc::new(Mutex::new(rx)),
        }
    }

    pub fn send(&self, value: T) {
        self.tx.lock().unwrap().send(value).unwrap();
    }

    pub fn extend(&self, values: impl IntoIterator<Item = T>) {
        let tx = self.tx.lock().unwrap();
        for value in values {
            tx.send(value).unwrap();
        }
    }

    pub fn next(&self) -> Option<T> {
        self.rx.lock().unwrap().try_recv().ok()
    }
}

struct Sandbox {
    renderer: Renderer,
    scene: Arc<RwLock<Scene>>,
    camera_controller: OrbitCameraController,
    camera_interaction_controller: OrbitCameraInteractionController,
    load_file_join: Option<JoinHandle<Vec<Box<dyn 'static + Send + Sync + ObjectData>>>>,
    ui_events: Combined<UiMessage>,
    render_events: Combined<RenderMessage>,
    default_material: Weak<Material>,
    selected: Option<u64>,
    total_ambient_lighting: Vec3,
    ui_add_light: bool,
    gizmo_mode: GizmoMode,
}

impl Sandbox {
    fn invoke_respond<T>(&mut self, respond: Respond<T>, data: T) {
        if let Some(respond) = respond {
            let mut ui_msg = vec![];
            let mut render_msg = vec![];
            respond(data, &mut ui_msg, &mut render_msg);
            self.ui_events.extend(ui_msg);
            self.render_events.extend(render_msg);
        }
    }

    fn process_render_messages(&mut self) -> Result<()> {
        while let Some(msg) = self.render_events.next() {
            match msg {
                RenderMessage::AddSphere {
                    radius,
                    nlon,
                    nlat,
                    respond,
                } => {
                    let sphere = MeshBuilder::new(Vertex::new).uv_sphere(radius, nlon, nlat)?;
                    let sphere = self.scene.write().unwrap().add_mesh(sphere);
                    self.invoke_respond(respond, sphere);
                }
                RenderMessage::CreateMaterial {
                    albedo,
                    normal,
                    rough_metal,
                    respond,
                } => {
                    let albedo = match albedo {
                        TextureDesc::Color(color) => TextureSlot::Color(color),
                        TextureDesc::Image(image) => {
                            TextureSlot::Texture(Texture::from_image(image.into_rgb32f())?)
                        }
                    };
                    let normal = if let Some(normal) = normal {
                        Some(Texture::from_image(normal.to_rgb32f())?)
                    } else {
                        None
                    };
                    let rough_metal = match rough_metal {
                        TextureDesc::Image(img) => {
                            let width = img.width().try_into()?;
                            let data = img
                                .into_rgb32f()
                                .pixels()
                                .flat_map(|px| {
                                    let [r, g, _] = px.0;
                                    [r, g]
                                })
                                .collect::<Vec<_>>();
                            let texture = Texture::from_2d_pixels(width, &data)?;
                            TextureSlot::Texture(texture)
                        }
                        TextureDesc::Color(color) => TextureSlot::Color(color),
                    };
                    let material = self.scene.write().unwrap().add_material(Material::create(
                        albedo,
                        normal,
                        rough_metal,
                    )?);
                    self.invoke_respond(respond, material);
                }
                RenderMessage::LoadMesh(model) => {
                    let mut scene = self.scene.write().unwrap();
                    match model.insert_into_scene(&mut scene) {
                        Ok(_) => {}
                        Err(err) => {
                            tracing::error!("Cannot insert into scene: {}", err)
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn process_ui_messages(&mut self) {
        while let Some(msg) = self.ui_events.next() {
            match msg {
                UiMessage::OpenMesh => {
                    let ui_events = self.ui_events.clone();
                    std::thread::spawn(move || {
                        let files = rfd::FileDialog::new()
                            .add_filter("Wavefront files", &["obj"])
                            .add_filter("All files", &["*"])
                            .pick_files();
                        let messages = files
                            .into_iter()
                            .flatten()
                            .map(|filepath| UiMessage::LoadMesh {
                                filepath,
                                respond: Some(Box::new(|obj_data, _, render| {
                                    render.push(RenderMessage::LoadMesh(obj_data))
                                })),
                            })
                            .collect::<Vec<_>>();
                        ui_events.extend(messages);
                    });
                }
                UiMessage::LoadMesh { filepath, respond } => {
                    match read_mesh::load_mesh_dynamic(filepath) {
                        Ok(data) => {
                            self.invoke_respond(respond, data);
                        }
                        Err(err) => {
                            let sources =
                                err.chain()
                                    .map(|src| format!("\t{}", src))
                                    .reduce(|mut str, s| {
                                        str.push_str(&s);
                                        str.push('\n');
                                        str
                                    }).unwrap_or("<No sources>".into());
                            tracing::error!("Error loading mesh: {}\n{}", err, sources)
                        }
                    }
                }
                UiMessage::AddLight { light, respond } => {
                    let light = self.scene.write().unwrap().add_light(light);
                    self.invoke_respond(respond, light);
                }
                UiMessage::InstanceLight { light, respond } => {
                    let id = self.scene.write().unwrap().instance_light(light).id();
                    self.invoke_respond(respond, id);
                }
                UiMessage::InstanceObject {
                    mesh,
                    material,
                    respond,
                } => {
                    let id = self
                        .scene
                        .write()
                        .unwrap()
                        .instance_object(material, mesh)
                        .id();
                    self.invoke_respond(respond, id);
                }
                UiMessage::Select(id) => {
                    self.selected.replace(id);
                }
                UiMessage::Deselect => {
                    self.selected.take();
                }
                UiMessage::DeleteInstance(id) => {
                    self.scene.write().unwrap().remove(id);
                }
            }
        }
    }

    fn poll_load_mesh_dialog(&mut self) {
        let load_file_done = self
            .load_file_join
            .as_ref()
            .map(|handle| handle.is_finished())
            .unwrap_or(false);
        if load_file_done {
            let handle = self.load_file_join.take().unwrap();
            for obj in handle.join().unwrap() {
                self.render_events.send(RenderMessage::LoadMesh(obj));
            }
        }
    }

    fn ui_menubar(&mut self, ctx: &UiContext) {
        egui::TopBottomPanel::top("top-menu").show(ctx.egui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    ui.menu_button("Add object", |ui| {
                        ui.menu_button("Sphere", |ui| {
                            thread_local! {
                                static SPHERE_RADIUS: RefCell<f32> = RefCell::new(1.);
                                static SPHERE_NLAT: RefCell<usize> = RefCell::new(12);
                                static SPHERE_NLON: RefCell<usize> = RefCell::new(24);
                            }
                            ui.label("Sphere");
                            num_value("Radius", &SPHERE_RADIUS, ui);
                            num_value("# slices latitude", &SPHERE_NLAT, ui);
                            num_value("# slices longitude", &SPHERE_NLON, ui);

                            if ui.button("Add").clicked() {
                                let radius = SPHERE_RADIUS.with(|cell| cell.borrow().clone());
                                let nlat = SPHERE_NLAT.with(|cell| cell.borrow().clone());
                                let nlon = SPHERE_NLON.with(|cell| cell.borrow().clone());
                                let default_material = self.default_material.clone();
                                self.render_events.send(RenderMessage::AddSphere {
                                    radius,
                                    nlon,
                                    nlat,
                                    respond: Some(Box::new(move |mesh, ui_msg, _| {
                                        ui_msg.push(UiMessage::InstanceObject {
                                            mesh: mesh.transformed(Transform::default()),
                                            material: default_material,
                                            respond: None,
                                        });
                                    })),
                                })
                            }
                        });
                        if ui.small_button("Load mesh...").clicked() {
                            self.ui_events.send(UiMessage::OpenMesh);
                        }
                    });
                    ui.toggle_value(&mut self.ui_add_light, "Add light ...");
                });
                self.camera_controller.ui_toolbar(ui);
                self.renderer.ui_toolbar(ui);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Max), |ui| {
                    let label = ui.label("Gizmo");
                    ui.radio_value(&mut self.gizmo_mode, GizmoMode::Translate, "Translate")
                        .labelled_by(label.id);
                    ui.radio_value(&mut self.gizmo_mode, GizmoMode::Rotate, "Rotate")
                        .labelled_by(label.id);
                    ui.radio_value(&mut self.gizmo_mode, GizmoMode::Scale, "Scale")
                        .labelled_by(label.id);
                })
            });
        });
    }

    fn objects_panel(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.heading("Objects");
            let table_builder = egui_extras::TableBuilder::new(ui)
                .columns(Column::auto(), 2)
                .column(Column::auto().resizable(true))
                .column(Column::auto())
                .column(Column::remainder().at_least(150.))
                .striped(true)
                .resizable(true);

            if let Some(selection) = self.selected {
                table_builder.scroll_to_row(selection as _, None)
            } else {
                table_builder
            }
            .header(20., |mut header| {
                header.col(|ui| ());
                header.col(|ui| ());
                header.col(|ui| {
                    ui.label(RichText::new("ID").strong());
                });
                header.col(|ui| {
                    ui.label(RichText::new("Type").strong());
                });
                header.col(|ui| {
                    ui.label(RichText::new("Name").strong());
                });
            })
            .body(|mut body| {
                for inst in self.scene.write().unwrap().instances_mut() {
                    body.row(20., |mut row| {
                        row.col(|ui| {
                            let mut checked =
                                self.selected.map(|sel| sel == inst.id()).unwrap_or(false);
                            if ui.checkbox(&mut checked, "").clicked() {
                                if checked {
                                    self.ui_events.send(UiMessage::Select(inst.id()));
                                } else {
                                    self.ui_events.send(UiMessage::Deselect);
                                }
                            }
                        });
                        row.col(|ui| {
                            ui.menu_button("+", |ui| {
                                if let Some(name) = &mut inst.name {
                                    let label = ui.label("Name");
                                    ui.add(egui::TextEdit::singleline(name))
                                        .labelled_by(label.id);
                                } else {
                                    if ui.small_button("Add name").clicked() {
                                        inst.named("");
                                    }
                                }
                                if ui.small_button("Delete").clicked() {
                                    self.ui_events.send(UiMessage::DeleteInstance(inst.id()));
                                }
                            });
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", inst.id()));
                        });
                        row.col(|ui| {
                            ui.label(match inst.entity() {
                                Entity::Light(..) => "Light",
                                Entity::Object(..) => "Object",
                                Entity::Camera(..) => "Camera",
                            });
                        });
                        row.col(|ui| {
                            if let Some(name) = &inst.name {
                                ui.label(name);
                            } else {
                                ui.colored_label(egui::Color32::GRAY, "<None>");
                            }
                        });
                    });
                }
            });
        });
    }
}

impl Application for Sandbox {
    fn window_features(wb: WindowBuilder) -> WindowBuilder {
        wb.with_inner_size(LogicalSize::new(1280, 860))
    }

    fn new(size: PhysicalSize<f32>) -> Result<Self> {
        let sizeu = UVec2::from_array(size.cast::<u32>().into());
        let mut renderer = Renderer::new(sizeu)?;
        let mut scene = Scene::new();
        let default_material =
            scene.add_material(Material::create([1., 1., 1.], None, [0.3, 0.3])?);
        let sphere = scene.add_mesh(MeshBuilder::new(Vertex::new).uv_sphere(1., 64, 32)?);
        scene.instance_object(
            default_material.clone(),
            sphere.transformed(Transform::default()),
        );
        let point_light = scene.add_light(Light::Point {
            color: Vec3::splat(3.5),
            position: Vec3::ZERO,
        });
        scene
            .instance_light(point_light.transformed(Transform::translation(vec3(2., 3., -1.))))
            .named("Point light");
        let mut camera_controller = OrbitCameraController::default();
        camera_controller.set_orientation(
            renderer.camera_mut(),
            vec2(30f32.to_radians(), 20f32.to_radians()),
        );
        Ok(Self {
            renderer,
            scene: Arc::new(RwLock::new(scene)),
            camera_controller,
            camera_interaction_controller: OrbitCameraInteractionController::default(),
            load_file_join: None,
            ui_events: Combined::new(),
            render_events: Combined::new(),
            default_material,
            selected: None,
            total_ambient_lighting: Vec3::ZERO,
            ui_add_light: false,
            gizmo_mode: GizmoMode::Translate,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) -> Result<()> {
        let sizef = Vec2::from_array(size.cast::<f32>().into());
        self.scene.write().unwrap().resize_cameras(sizef);
        self.renderer
            .resize(UVec2::from_array(size.cast::<u32>().into()))?;
        Ok(())
    }

    fn interact(&mut self, event: WindowEvent) -> Result<()> {
        self.camera_interaction_controller.dispatch_event(
            &mut self.camera_controller,
            self.renderer.camera_mut(),
            event,
        );
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn tick(&mut self, ctx: TickContext) -> Result<()> {
        self.process_ui_messages();
        self.poll_load_mesh_dialog();
        self.camera_controller
            .update(ctx.dt, self.renderer.camera_mut());

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn render(&mut self, ctx: RenderContext) -> Result<()> {
        self.process_render_messages()?;

        let mut scene = self.scene.write().unwrap();
        if let Some(light_buffer) = scene.updated_light_buffer() {
            let light_buffer = light_buffer?;
            self.renderer.set_light_buffer(light_buffer);
            self.total_ambient_lighting = scene
                .lights()
                .filter_map(|l| match l {
                    Light::Ambient { color } => Some(color),
                    _ => None,
                })
                .sum::<Vec3>();
        }

        self.renderer
            .begin_render(self.total_ambient_lighting.extend(1.))?;
        for (material, mesh) in scene.objects() {
            self.renderer.submit_mesh(material, mesh);
        }
        self.renderer.flush(ctx.dt)
    }

    fn ui(&mut self, ctx: UiContext) {
        self.ui_menubar(&ctx);
        self.renderer.ui(ctx.egui);
        self.camera_controller.ui(ctx.egui);

        egui::SidePanel::left("left-panel").show(ctx.egui, |ui| {
            self.objects_panel(ui);
        });

        egui::Window::new("Add light").resizable(true).open(&mut self.ui_add_light).show(ctx.egui, |ui| {
            thread_local! {
                static NEW_LIGHT: RefCell<NewLight> = RefCell::new(NewLight {color: [1.; 3], ty: NewLightType::Point, transform: Transform::default() });
            }
            NEW_LIGHT.with(|key| {
                ui.label("Type");
                ui.vertical(|ui| {
                    let ty = &mut key.borrow_mut().ty;
                    ui.radio_value(ty, NewLightType::Point, "Point");
                    ui.radio_value(ty, NewLightType::Directional, "Directional");
                    ui.radio_value(ty, NewLightType::Ambient, "Ambient");
                });

                ui.horizontal(|ui| {
                    let color = ui.label("Color");
                    ui.color_edit_button_rgb(&mut key.borrow_mut().color).labelled_by(color.id);
                });

                if ui.button("Add").clicked() {
                    let new_light = key.borrow().clone();
                    let color = Vec3::from_array(new_light.color);
                    let light = match new_light.ty {
                        NewLightType::Ambient => Light::Ambient {color },
                        NewLightType::Point => Light::Point {color, position: new_light.transform.position },
                        NewLightType::Directional => Light::Directional {color, dir: new_light.transform.backward()},
                    };
                    self.ui_events.send(UiMessage::AddLight {light,respond: Some(Box::new(move |light, ui, _| ui.push(UiMessage::InstanceLight {light: light.transformed(new_light.transform), respond:None})))})
                }
            });
        });

        let mut scene = self.scene.write().unwrap();
        let camera = self.renderer.camera_mut();
        if let Some(inst) = self.selected.and_then(|i| scene.get_mut(i)) {
            egui::CentralPanel::default()
                .frame(egui::Frame::none())
                .show(ctx.egui, |ui| {
                    let gizmo = Gizmo::new("manipulator")
                        .view_matrix(camera.transform.matrix().to_cols_array_2d())
                        .projection_matrix(camera.projection.matrix().to_cols_array_2d())
                        .model_matrix(inst.transform.matrix().to_cols_array_2d())
                        .mode(self.gizmo_mode);
                    if let Some(response) = gizmo.interact(ui) {
                        inst.transform =
                            Transform::from_matrix(Mat4::from_cols_array_2d(&response.transform));
                    }
                });
        }
    }
}

fn num_value<T: Clone + Numeric>(
    name: &'static str,
    value: &'static std::thread::LocalKey<RefCell<T>>,
    ui: &mut Ui,
) -> Response {
    ui.horizontal(|ui| {
        let label = ui.label(name);
        value.with(|cell| {
            ui.add(egui::DragValue::new(&mut *cell.borrow_mut()))
                .labelled_by(label.id)
        })
    })
    .inner
}

fn main() -> Result<()> {
    rose_platform::run::<Sandbox>("Sandbox")
}