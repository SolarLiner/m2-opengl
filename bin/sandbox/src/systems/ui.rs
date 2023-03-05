use std::{
    any::TypeId, cell::RefCell, collections::HashMap, collections::HashSet, f32::consts::PI,
    marker::PhantomData,
};

use assets_manager::{
    AnyCache,
    Asset,
    asset::DirLoadable, Handle, SharedString, source::{DirEntry, Source},
};
use egui::{
    Align, Color32, Context, DragValue, Grid, Layout, PointerButton, RichText, Sense, TextEdit, Ui,
    WidgetText,
};
use egui_dock::{NodeIndex, TabViewer, Tree};
use egui_gizmo::{Gizmo, GizmoMode};
use glam::{EulerRot, Mat4, Quat, Vec2, vec3, Vec3};
use hecs::{CommandBuffer, Component, Entity, EntityRef};

use rose_core::transform::Transform;

use crate::{
    assets::{material::Material, mesh::MeshAsset, scene::NamedObject},
    scene::Scene,
};
use crate::systems::render::RenderSystem;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Tabs {
    SceneHierarchy,
    Inspector,
    Viewport,
    Assets,
    Postprocessing,
    CameraDebug,
    RendererDebug,
}

impl Tabs {
    pub const ALL: [Tabs; 7] = [
        Self::SceneHierarchy,
        Self::Inspector,
        Self::Viewport,
        Self::Assets,
        Self::Postprocessing,
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
            Self::Postprocessing => "Post-processing".to_string(),
            Self::CameraDebug => "Camera debug".to_string(),
            Self::RendererDebug => "Renderer debug".to_string(),
        }
    }
}

pub struct UiSystem {
    pub last_state: UiState,
    pub gizmo_mode: GizmoMode,
    tabs: Tree<Tabs>,
    selected_entity: Option<Entity>,
    component_ui_registry: HashMap<TypeId, DynComponentUi>,
    spawn_component: Vec<DynInsertComponent>,
}

impl UiSystem {
    pub fn new() -> Self {
        let mut tabs = Tree::new(vec![Tabs::Viewport]);
        let [main, left] = tabs.split_left(NodeIndex::root(), 0.2, vec![Tabs::SceneHierarchy]);
        tabs.split_right(main, 0.8, vec![Tabs::Assets]);
        tabs.split_below(left, 0.5, vec![Tabs::Inspector]);
        Self {
            last_state: UiState::default(),
            gizmo_mode: GizmoMode::Translate,
            tabs,
            selected_entity: None,
            component_ui_registry: HashMap::new(),
            spawn_component: vec![],
        }
    }

    pub fn register_component_ui<C: ComponentUi>(mut self) -> Self {
        self.component_ui_registry
            .insert(TypeId::of::<C>(), &component_ui::<C>);
        self
    }

    pub fn register_spawn_component<C: NamedComponent + Default>(mut self) -> Self {
        self.spawn_component.push(&insert_component::<C>);
        self
    }

    pub fn on_ui(&mut self, ctx: &Context, scene: Option<&Scene>, renderer: &mut RenderSystem) {
        if scene.is_none() {
            self.selected_entity.take();
        }
        let mut state = UiStateLocal::new(
            scene,
            self.gizmo_mode,
            &mut self.selected_entity,
            &self.component_ui_registry,
            &self.spawn_component,
            renderer,
        );
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                egui_dock::DockArea::new(&mut self.tabs)
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
        for (node, tab) in state.new_nodes.drain(..) {
            self.tabs.set_focused_node(node);
            self.tabs.push_to_focused_leaf(tab);
        }
        self.last_state = state.state;
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
    new_nodes: Vec<(NodeIndex, Tabs)>,
    scene: Option<&'a Scene>,
    gizmo_mode: GizmoMode,
    selected_entity: &'a mut Option<Entity>,
    component_ui_registry: &'a HashMap<TypeId, DynComponentUi>,
    spawn_component: &'a [DynInsertComponent],
    renderer: &'a mut RenderSystem,
}

impl<'a> UiStateLocal<'a> {
    fn new(
        scene: Option<&'a Scene>,
        gizmo_mode: GizmoMode,
        selected_entity: &'a mut Option<Entity>,
        component_ui_registry: &'a HashMap<TypeId, DynComponentUi>,
        spawn_component: &'a [DynInsertComponent],
        renderer: &'a mut RenderSystem,
    ) -> Self {
        Self {
            state: UiState::default(),
            new_nodes: vec![],
            gizmo_mode,
            scene,
            selected_entity,
            component_ui_registry,
            spawn_component,
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
                            let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
                            let gizmo_interaction = if let Some(entity) = *self.selected_entity {
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
                                self.state.mouse_buttons =
                                    (response.dragged_by(PointerButton::Primary), response.dragged_by(PointerButton::Secondary));
                                self.state.mouse_delta = glam::vec2(drag.x, drag.y);
                                self.state.mouse_scroll = input.scroll_delta.y;
                                tracing::debug!(state=?self.state);
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
                                    let selected = *self.selected_entity == Some(entity.entity());
                                    let label_resp =
                                        ui.selectable_label(selected, name).context_menu(|ui| {
                                            if let Some(mut name) = entity.get::<&mut String>() {
                                                let name_label = ui.label("Name:").id;
                                                ui.text_edit_singleline(&mut *name)
                                                    .labelled_by(name_label);
                                                ui.separator();
                                                if ui.small_button("Remove").clicked() {
                                                    cmd.despawn(entity.entity());
                                                    ui.close_menu();
                                                }
                                            }
                                        });
                                    if label_resp.clicked() {
                                        self.selected_entity.replace(entity.entity());
                                    }
                                }
                            });
                            let size = ui.available_size();
                            let (_, response) = ui.allocate_exact_size(size, Sense::click());
                            if response.clicked() {
                                self.selected_entity.take();
                            }
                        });
                    } else {
                        ui.monospace("No loaded scene");
                    }
                });
            }
            Tabs::Inspector => {
                if let Some(scene) = self.scene {
                    scene.with_world(|world, cmd| {
                        if let Some(entity) = *self.selected_entity {
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
                                    for spawn_cmp in self.spawn_component {
                                        spawn_cmp(ui, eref, cmd);
                                    }
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
                                    } else {
                                        if ui.button("Add name").labelled_by(name_label).clicked() {
                                            cmd.insert_one(eref.entity(), String::from(""));
                                        }
                                    }
                                    ui.end_row();
                                });

                            for type_id in eref.component_types() {
                                if let Some(component_ui) = self.component_ui_registry.get(&type_id)
                                {
                                    component_ui(ui, eref, cmd);
                                }
                            }
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
                                            ui.monospace(&id[1..]).context_menu(|ui| {
                                                ui.set_max_width(250.);
                                                if let Some(entity) = *self.selected_entity {
                                                    if ui.small_button("Add mesh component").clicked() {
                                                        match cache.load::<MeshAsset>(id.as_str()) {
                                                            Ok(mat) => cmd.insert_one(entity, mat),
                                                            Err(err) => tracing::error!("Could not load {:?} as mesh: {}", id.as_str(), err),
                                                        }
                                                    }
                                                    if ui.small_button("Add material component").clicked() {
                                                        match cache.load::<Material>(id.as_str()) {
                                                            Ok(mat) => cmd.insert_one(entity, mat),
                                                            Err(err) => tracing::error!("Could not load {:?} as material: {}", id.as_str(), err),
                                                        }
                                                    }
                                                    ui.separator();
                                                }
                                                if ui.small_button("New entity with this mesh").clicked() {
                                                    match cache.load::<MeshAsset>(id.as_str()) {
                                                        Ok(handle) => cmd.spawn((Transform::default(), handle)),
                                                        Err(err) => tracing::error!("Could not load {:?} as mesh: {}", id.as_str(), err),
                                                    }
                                                }
                                                if ui.small_button("New entity with this material").clicked() {
                                                    match cache.load::<Material>(id.as_str()) {
                                                        Ok(handle) => cmd.spawn((Transform::default(), handle)),
                                                        Err(err) => tracing::error!("Could not load {:?} as material: {}", id.as_str(), err),
                                                    }
                                                }
                                            });
                                            ui.add_enabled_ui(false, |ui| {
                                                ui.checkbox(&mut used.contains(id.as_str()), "Used")
                                            });
                                            ui.end_row();
                                        }
                                    });
                                });
                            });
                    });
                }
            }
            Tabs::Postprocessing => {
                let pp_iface = self.renderer.renderer.post_process_interface();
                pp_iface.ui(ui);
            }
            Tabs::CameraDebug => {
                ui.collapsing("Camera", |ui| {
                    let camera = &self.renderer.camera;
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
            if let DirEntry::File(id, _) = entry {
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
            if let DirEntry::File(id, ext) = entry {
                if A::EXTENSIONS.contains(&ext) {
                    ids.push(id.into());
                }
            }
        })?;
        Ok(ids)
    }
}

pub trait NamedComponent: Component {
    const NAME: &'static str;
}

pub trait ComponentUi: NamedComponent + Component {
    fn ui(&mut self, ui: &mut Ui);
}

impl NamedComponent for Handle<'static, MeshAsset> {
    const NAME: &'static str = "Mesh";
}

impl NamedComponent for Handle<'static, Material> {
    const NAME: &'static str = "Material";
}

impl<A> ComponentUi for Handle<'static, A>
    where
        Self: NamedComponent,
{
    fn ui(&mut self, ui: &mut Ui) {
        Grid::new("material-handle").num_columns(2).show(ui, |ui| {
            let handle_label = ui.label("Handle").id;
            ui.label(RichText::new(self.id().as_str()).strong().monospace())
                .labelled_by(handle_label);
        });
    }
}

impl NamedComponent for Transform {
    const NAME: &'static str = "Transform";
}

impl ComponentUi for Transform {
    fn ui(&mut self, ui: &mut Ui) {
        let ui_pos3 = |ui: &mut Ui, v: &mut Vec3| {
            let pos_label = ui.label("Position").id;
            ui.horizontal(|ui| {
                ui.add(DragValue::new(&mut v.x).prefix("X: ").suffix(" m"));
                ui.add(DragValue::new(&mut v.y).prefix("Y: ").suffix(" m"));
                ui.add(DragValue::new(&mut v.z).prefix("Z: ").suffix(" m"));
            })
                .response
                .labelled_by(pos_label);
        };
        let ui_rot3 = |ui: &mut Ui, v: &mut Quat| {
            let (a, b, c) = v.to_euler(EulerRot::ZYX);
            let mut rot_v3 = vec3(c, b, a) * 180. / PI;
            let pos_label = ui.label("Rotation").id;
            ui.horizontal(|ui| {
                ui.add(DragValue::new(&mut rot_v3.x).prefix("X: ").suffix(" °"));
                ui.add(DragValue::new(&mut rot_v3.y).prefix("Y: ").suffix(" °"));
                ui.add(DragValue::new(&mut rot_v3.z).prefix("Z: ").suffix(" °"));
            })
                .response
                .labelled_by(pos_label);
            let rot_v3 = rot_v3 * PI / 180.;
            let [b, c, a] = rot_v3.to_array();
            *v = Quat::from_euler(EulerRot::ZYX, a, b, c);
        };
        let ui_scale3 = |ui: &mut Ui, v: &mut Vec3| {
            let pos_label = ui.label("Scale").id;
            ui.horizontal(|ui| {
                *v *= 100.;
                ui.add(DragValue::new(&mut v.x).prefix("X: ").suffix(" %"));
                ui.add(DragValue::new(&mut v.y).prefix("Y: ").suffix(" %"));
                ui.add(DragValue::new(&mut v.z).prefix("Z: ").suffix(" %"));
                *v /= 100.;
            })
                .response
                .labelled_by(pos_label);
        };

        Grid::new("selected-entity-transform")
            .num_columns(2)
            .show(ui, |ui| {
                ui_pos3(ui, &mut self.position);
                ui.end_row();

                ui_rot3(ui, &mut self.rotation);
                ui.end_row();

                ui_scale3(ui, &mut self.scale);
                // ui.end_row();
            });
    }
}

fn component_ui<T: ComponentUi>(ui: &mut Ui, entity: EntityRef, cmd: &mut CommandBuffer) {
    if let Some(mut component) = entity.get::<&mut T>() {
        ui.collapsing(T::NAME, |ui| component.ui(ui))
            .header_response
            .context_menu(|ui| {
                if ui.small_button("Remove").clicked() {
                    cmd.remove_one::<T>(entity.entity());
                }
            });
    }
}

type DynComponentUi = &'static (dyn Send + Sync + Fn(&mut Ui, EntityRef<'_>, &mut CommandBuffer));

fn insert_component<C: NamedComponent + Default>(
    ui: &mut Ui,
    entity: EntityRef,
    cmd: &mut CommandBuffer,
) {
    if ui.small_button(C::NAME).clicked() {
        cmd.insert_one(entity.entity(), C::default());
    }
}

type DynInsertComponent =
&'static (dyn Fn(&mut Ui, EntityRef<'_>, &mut CommandBuffer) + Send + Sync);
