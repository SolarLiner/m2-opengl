use std::{cell::RefCell, collections::HashSet, marker::PhantomData};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use egui::{
    Align, Color32, Context, DragValue, Grid, Layout, PointerButton, Sense, TextEdit, Ui,
    WidgetText,
};
use egui_dock::{NodeIndex, TabViewer, Tree};
use egui_gizmo::{Gizmo, GizmoMode};
use glam::{Mat4, Vec2};

use rose_core::transform::Transform;
use rose_ecs::assets::{Material, MeshAsset, NamedObject, ObjectBundle};
use rose_ecs::CoreSystems;
use rose_ecs::prelude::*;
use rose_ecs::prelude::asset::DirLoadable;
use rose_ecs::systems::{RenderSystem, UiSystem};
use rose_renderer::env::{EnvironmentMap, SimpleSky, SimpleSkyParams};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Tabs {
    SceneHierarchy,
    Inspector,
    Viewport,
    Assets,
    Environment,
    Postprocessing,
    CameraDebug,
    RendererDebug,
}

impl Tabs {
    pub const ALL: [Tabs; 8] = [
        Self::SceneHierarchy,
        Self::Inspector,
        Self::Viewport,
        Self::Assets,
        Self::Postprocessing,
        Self::Environment,
        Self::CameraDebug,
        Self::RendererDebug,
    ];
}

impl ToString for Tabs {
    fn to_string(&self) -> String {
        match self {
            Self::SceneHierarchy => "Scene hierarchy".to_string(),
            Self::Inspector => "Inspector".to_string(),
            Self::Viewport => "Viewport".to_string(),
            Self::Assets => "Assets".to_string(),
            Self::Environment => "Environment".to_string(),
            Self::Postprocessing => "Post-processing".to_string(),
            Self::CameraDebug => "Camera debug".to_string(),
            Self::RendererDebug => "Renderer debug".to_string(),
        }
    }
}

pub struct EditorUiSystem {
    pub last_state: UiState,
    pub gizmo_mode: GizmoMode,
    core_system: UiSystem,
    tabs: Arc<Mutex<Tree<Tabs>>>,
    selected_entity: Option<Entity>,
    envmap_path: Option<PathBuf>,
}

impl EditorUiSystem {
    pub fn new() -> Self {
        let mut tabs = Tree::new(vec![Tabs::Viewport]);
        let [main, left] = tabs.split_left(NodeIndex::root(), 0.2, vec![Tabs::SceneHierarchy]);
        tabs.split_right(main, 0.8, vec![Tabs::Assets]);
        tabs.split_below(left, 0.5, vec![Tabs::Inspector]);
        let mut core_system = UiSystem::new();
        core_system
            .register_component::<Transform>()
            .register_component::<Active>()
            .register_component::<Inactive>()
            .register_component::<CameraParams>()
            .register_component::<PanOrbitCamera>()
            .register_component::<Handle<'static, MeshAsset>>()
            .register_component::<Handle<'static, Material>>()
            .register_component::<Light>()
            .register_component::<SceneId>()
            .register_spawn::<Transform>()
            .register_spawn::<Active>()
            .register_spawn::<Inactive>()
            .register_spawn::<CameraParams>()
            .register_spawn::<PanOrbitCamera>()
            .register_spawn::<Light>();
        Self {
            last_state: UiState::default(),
            gizmo_mode: GizmoMode::Translate,
            core_system,
            tabs: Arc::new(Mutex::new(tabs)),
            selected_entity: None,
            envmap_path: None,
        }
    }

    pub fn on_ui(&mut self, ctx: &Context, scene: Option<&Scene>, core: &mut CoreSystems) {
        if scene.is_none() {
            self.selected_entity.take();
        }
        let (state, new_nodes) = {
            let tabs = self.tabs.clone();
            let mut state = UiStateLocal::new(scene, self, self.gizmo_mode, &mut core.render);
            egui::CentralPanel::default()
                .frame(egui::Frame::none())
                .show(ctx, |ui| {
                    egui_dock::DockArea::new(&mut tabs.lock().unwrap())
                        .style({
                            let mut style = egui_dock::Style::from_egui(ctx.style().as_ref());
                            style.show_add_popup = true;
                            style.show_add_buttons = true;
                            style.show_context_menu = true;
                            style.border_color = Color32::TRANSPARENT;
                            style
                        })
                        .show_inside(ui, &mut state);
                });
            (state.state, state.new_nodes)
        };
        for (node, tab) in new_nodes {
            let mut tabs = self.tabs.lock().unwrap();
            tabs.set_focused_node(node);
            tabs.push_to_focused_leaf(tab);
        }
        self.last_state = state;
    }
}

#[derive(Debug, Copy, Clone)]
pub struct UiState {
    pub mouse_delta: Vec2,
    pub mouse_scroll: f32,
    pub mouse_buttons: (bool, bool),
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            mouse_buttons: (false, false),
            mouse_scroll: 0.,
            mouse_delta: Vec2::ZERO,
        }
    }
}

struct UiStateLocal<'a> {
    state: UiState,
    system: &'a mut EditorUiSystem,
    new_nodes: Vec<(NodeIndex, Tabs)>,
    scene: Option<&'a Scene>,
    gizmo_mode: GizmoMode,
    renderer: &'a mut RenderSystem,
}

impl<'a> UiStateLocal<'a> {
    fn new(
        scene: Option<&'a Scene>,
        system: &'a mut EditorUiSystem,
        gizmo_mode: GizmoMode,
        renderer: &'a mut RenderSystem,
    ) -> Self {
        Self {
            state: UiState::default(),
            system,
            new_nodes: vec![],
            gizmo_mode,
            scene,
            renderer,
        }
    }
}

impl<'a> TabViewer for UiStateLocal<'a> {
    type Tab = Tabs;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match tab {
            Tabs::Viewport => {
                egui::Frame::none()
                    .fill(Color32::TRANSPARENT)
                    .inner_margin(0.)
                    .outer_margin(0.)
                    .show(ui, |ui| {
                        if let Some(scene) = self.scene {
                            let size = ui.available_size_before_wrap();
                            let (rect, response) =
                                ui.allocate_exact_size(size, Sense::click_and_drag());
                            let gizmo_interaction = if let Some(entity) =
                                self.system.selected_entity
                            {
                                scene.with_world(|world, _| {
                                    let eref = match world.entity(entity) {
                                        Ok(eref) => eref,
                                        Err(_) => {
                                            // Entity was just deleted, silently absorbing error
                                            return false;
                                        }
                                    };
                                    if let Some(mut tr) = eref.get::<&mut Transform>() {
                                        ui.scope(|ui| {
                                            let camera = &self.renderer.camera;
                                            let gizmo_interact =
                                                Gizmo::new("selected-entity-gizmo")
                                                    .viewport(rect)
                                                    .model_matrix(tr.matrix().to_cols_array_2d())
                                                    .view_matrix(
                                                        camera
                                                            .transform
                                                            .matrix()
                                                            .to_cols_array_2d(),
                                                    )
                                                    .projection_matrix(
                                                        camera
                                                            .projection
                                                            .matrix()
                                                            .to_cols_array_2d(),
                                                    )
                                                    .mode(self.gizmo_mode)
                                                    .interact(ui);
                                            if let Some(interact) = gizmo_interact {
                                                *tr = Transform::from_matrix(
                                                    Mat4::from_cols_array_2d(&interact.transform),
                                                );
                                                true
                                            } else {
                                                false
                                            }
                                        })
                                            .inner
                                    } else {
                                        false
                                    }
                                })
                            } else {
                                false
                            };
                            if !gizmo_interaction {
                                let input = ui.input();
                                let drag = input.pointer.delta();
                                self.state.mouse_buttons = (
                                    response.dragged_by(PointerButton::Primary),
                                    response.dragged_by(PointerButton::Secondary),
                                );
                                self.state.mouse_delta = glam::vec2(drag.x, drag.y);
                                self.state.mouse_scroll = input.scroll_delta.y;
                            }
                        }
                    });
            }
            Tabs::SceneHierarchy => {
                egui::Frame::none().show(ui, |ui| {
                    if let Some(scene) = self.scene {
                        ui.vertical(|ui| {
                            scene.with_world(|world, cmd| {
                                let mut q = world.query::<()>();
                                for (entity, _) in q.iter() {
                                    let entity = world.entity(entity).unwrap();
                                    let name = entity
                                        .get::<&String>()
                                        .map(|s| s.to_string())
                                        .or_else(|| {
                                            entity.get::<&NamedObject>().map(|n| {
                                                format!("[Object {:?}]", n.object.as_str())
                                            })
                                        })
                                        .unwrap_or("<Unnamed>".to_string());
                                    let selected =
                                        self.system.selected_entity == Some(entity.entity());
                                    let label_resp =
                                        ui.selectable_label(selected, name).context_menu(|ui| {
                                            if let Some(mut name) = entity.get::<&mut String>() {
                                                let name_label = ui.label("Name:").id;
                                                ui.text_edit_singleline(&mut *name)
                                                    .labelled_by(name_label);
                                            } else {
                                                if ui.small_button("Add name").clicked() {
                                                    cmd.insert_one(entity.entity(), String::new());
                                                }
                                            }
                                            ui.separator();
                                            if ui.small_button("Remove").clicked() {
                                                cmd.despawn(entity.entity());
                                                ui.close_menu();
                                            }
                                        });
                                    if label_resp.clicked() {
                                        self.system.selected_entity.replace(entity.entity());
                                    }
                                }
                                let size = ui.available_size();
                                let (_, response) = ui.allocate_exact_size(size, Sense::click());
                                if response
                                    .context_menu(|ui| {
                                        if ui.small_button("Add empty").clicked() {
                                            cmd.spawn(());
                                            ui.close_menu();
                                        }
                                    })
                                    .clicked()
                                {
                                    self.system.selected_entity.take();
                                }
                            });
                        });
                    } else {
                        ui.monospace("No loaded scene");
                    }
                });
            }
            Tabs::Inspector => {
                if let Some(scene) = self.scene {
                    scene.with_world(|world, cmd| {
                        if let Some(entity) = self.system.selected_entity {
                            // let eref = world.entity(entity).unwrap();
                            let eref = match world.entity(entity) {
                                Ok(eref) => eref,
                                Err(_) => {
                                    // Entity *just* got deleted, ignoring
                                    return;
                                }
                            };
                            ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                                ui.menu_button("+", |ui| {
                                    self.system.core_system.components_ui(ui, eref, cmd)
                                });
                            });
                            Grid::new("selected-entity-properties")
                                .num_columns(2)
                                .show(ui, |ui| {
                                    let id_label = ui.label("Entity ID").id;
                                    ui.monospace(eref.entity().id().to_string())
                                        .labelled_by(id_label);
                                    ui.end_row();

                                    let name_label = ui.label("Name").id;
                                    if let Some(mut name) = eref.get::<&mut String>() {
                                        ui.text_edit_singleline(&mut *name).labelled_by(name_label);
                                    } else if ui
                                        .button("Add name")
                                        .labelled_by(name_label)
                                        .clicked()
                                    {
                                        cmd.insert_one(eref.entity(), String::from(""));
                                    }
                                    ui.end_row();
                                });

                            self.system.core_system.components_ui(ui, eref, cmd);
                        }
                    });
                } else {
                    ui.monospace("No loaded scene");
                }
            }
            Tabs::Assets => {
                if let Some(scene) = self.scene {
                    let cache = scene.asset_cache();
                    let dir_handle = cache.load_dir::<AnyDirLoader>(".", true).unwrap();
                    let used: HashSet<SharedString> = scene.with_world(|world, _| {
                        world
                            .query::<&Handle<MeshAsset>>()
                            .iter()
                            .map(|(_, h)| h.id().clone())
                            .chain(
                                world
                                    .query::<&Handle<Material>>()
                                    .iter()
                                    .map(|(_, h)| h.id().clone()),
                            )
                            .collect::<HashSet<_>>()
                    });
                    ui.add_enabled_ui(false, |ui| {
                        let mut enabled = cache.is_hot_reloaded();
                        ui.checkbox(&mut enabled, "Hot reload");
                    });
                    scene.with_world(|_, cmd| {
                        thread_local! {static SEARCH: RefCell<String> = RefCell::new(String::new());}
                        ui.horizontal(|ui| {
                            let search_label = ui.label("Search").id;
                            SEARCH.with(|key| {
                                let size = ui.available_size_before_wrap();
                                let width = size.x;
                                ui.add_sized([width - 20., 20.], TextEdit::singleline(&mut *key.borrow_mut())).labelled_by(search_label);
                                if ui.button("x").clicked() {
                                    key.borrow_mut().clear();
                                }
                            });
                        });
                        egui::ScrollArea::new([false, true])
                            .always_show_scroll(true)
                            .hscroll(false)
                            .show(ui, |ui| {
                                Grid::new("asset-list").num_columns(2).show(ui, |ui| {
                                    SEARCH.with(|key| {
                                        for id in dir_handle.ids().filter(|id| id.contains(&*key.borrow())) {
                                            let id = &id[2..];
                                            ui.monospace(id).context_menu(|ui| {
                                                ui.set_max_width(250.);
                                                if let Some(entity) = self.system.selected_entity {
                                                    if ui.small_button("Add mesh component").clicked() {
                                                        match cache.load::<MeshAsset>(id) {
                                                            Ok(mat) => cmd.insert_one(entity, mat),
                                                            Err(err) => tracing::error!("Could not load {:?} as mesh: {}", id, err),
                                                        }
                                                    }
                                                    if ui.small_button("Add material component").clicked() {
                                                        match cache.load::<Material>(id) {
                                                            Ok(mat) => cmd.insert_one(entity, mat),
                                                            Err(err) => tracing::error!("Could not load {:?} as material: {}", id, err),
                                                        }
                                                    }
                                                    ui.separator();
                                                }
                                                if ui.small_button("New entity with this object").clicked() {
                                                    match ObjectBundle::from_asset_cache(cache, Transform::default(), id) {
                                                        Ok(bundle) => {
                                                            cmd.spawn(bundle);
                                                        }
                                                        Err(err) => {
                                                            tracing::error!("Could not load {:?} as an object: {}", id, err);
                                                        }
                                                    }
                                                }
                                                if ui.small_button("New entity with this mesh").clicked() {
                                                    match cache.load::<MeshAsset>(id) {
                                                        Ok(handle) => cmd.spawn(ObjectBundle {
                                                            transform: Transform::default(),
                                                            active: Active,
                                                            mesh: handle,
                                                            material: self.renderer.default_material_handle(scene.asset_cache()),
                                                        }),
                                                        Err(err) => tracing::error!("Could not load {:?} as mesh: {}", id, err),
                                                    }
                                                }
                                                if ui.small_button("New entity with this material").clicked() {
                                                    match cache.load::<Material>(id) {
                                                        Ok(handle) => cmd.spawn(ObjectBundle {
                                                            transform: Transform::default(),
                                                            active: Active,
                                                            mesh: self.renderer.primitive_sphere(scene.asset_cache()),
                                                            material: handle,
                                                        }),
                                                        Err(err) => tracing::error!("Could not load {:?} as material: {}", id, err),
                                                    }
                                                }
                                            });
                                            ui.add_enabled_ui(false, |ui| {
                                                ui.checkbox(&mut used.contains(id), "Used")
                                            });
                                            ui.end_row();
                                        }
                                    });
                                });
                            });
                    });
                }
            }
            Tabs::Environment => {
                ui.collapsing("Environment map", |ui| {
                    let mut remove_path = false;
                    if let Some(path) = self.system.envmap_path.as_mut() {
                        ui.horizontal(|ui| {
                            if ui.button("X").clicked() {
                                self.renderer.renderer.set_environment(
                                    SimpleSky::new(SimpleSkyParams::default()).unwrap(),
                                );
                                remove_path = true;
                            }
                            if ui.button("Change").clicked() {
                                if let Some(new_path) = rfd::FileDialog::new()
                                    .add_filter(
                                        "Images",
                                        &["jpg", "png", "bmp", "exr", "hdr", "tif", "tga"],
                                    )
                                    .pick_file()
                                {
                                    self.renderer
                                        .renderer
                                        .set_environment(EnvironmentMap::load(&new_path).unwrap());
                                    path.clone_from(&new_path);
                                }
                            }
                        })
                            .response;
                        ui.end_row();
                    } else {
                        if ui.button("Open").clicked() {
                            if let Some(new_path) = rfd::FileDialog::new()
                                .add_filter(
                                    "Images",
                                    &["jpg", "png", "bmp", "exr", "hdr", "tif", "tga"],
                                )
                                .pick_file()
                            {
                                self.renderer
                                    .renderer
                                    .set_environment(EnvironmentMap::load(&new_path).unwrap());
                                self.system.envmap_path.replace(new_path);
                            }
                        }
                    }
                    if remove_path {
                        self.system.envmap_path.take();
                    }
                });
                if let Some(simple_sky) = self.renderer.renderer.environment_mut::<SimpleSky>() {
                    ui.collapsing("Simple sky parameters", |ui| simple_sky.params.ui(ui));
                }
            }
            Tabs::Postprocessing => {
                let pp_iface = self.renderer.renderer.post_process_interface();
                pp_iface.ui(ui);
            }
            Tabs::CameraDebug => {
                ui.collapsing("Camera", |ui| {
                    let camera = &mut self.renderer.camera;
                    Grid::new("camera-debug-params")
                        .num_columns(2)
                        .show(ui, |ui| {
                            let fov_label = ui.label("FOV").id;
                            ui.add(
                                DragValue::new(&mut camera.projection.fovy)
                                    .suffix(" rad")
                                    .speed(0.1),
                            )
                                .labelled_by(fov_label);
                        });
                    ui.monospace(format!("{:#?}", camera.transform));
                    ui.monospace(format!("{:#?}", camera.projection));
                });
            }
            Tabs::RendererDebug => {
                ui.collapsing("Debug", |ui| {
                    self.renderer.renderer.ui_debug_panel(ui);
                });
                ui.collapsing("Statistics", |ui| {
                    self.renderer.renderer.ui_render_stats(ui);
                });
            }
        }
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.to_string().into()
    }

    fn add_popup(&mut self, ui: &mut Ui, node: NodeIndex) {
        ui.set_width(150.);
        for tab in Tabs::ALL {
            if ui.small_button(tab.to_string()).clicked() {
                self.new_nodes.push((node, tab));
            }
        }
    }

    fn clear_background(&self, tab: &Self::Tab) -> bool {
        !matches!(tab, Tabs::Viewport)
    }
}

struct AnyDirLoader;

impl DirLoadable for AnyDirLoader {
    fn select_ids(cache: AnyCache, id: &SharedString) -> std::io::Result<Vec<SharedString>> {
        let source = cache.source();
        let mut ids = vec![];
        source.read_dir(id.as_str(), &mut |entry| {
            if let source::DirEntry::File(id, _) = entry {
                ids.push(id.into());
            }
        })?;
        Ok(ids)
    }
}

struct SingleAssetDirLoader<A>(PhantomData<A>);

impl<A: Asset> DirLoadable for SingleAssetDirLoader<A> {
    fn select_ids(cache: AnyCache, id: &SharedString) -> std::io::Result<Vec<SharedString>> {
        let source = cache.source();
        let mut ids = vec![];
        source.read_dir(id.as_str(), &mut |entry| {
            if let source::DirEntry::File(id, ext) = entry {
                if A::EXTENSIONS.contains(&ext) {
                    ids.push(id.into());
                }
            }
        })?;
        Ok(ids)
    }
}
