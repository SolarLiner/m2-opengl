use std::cell::RefCell;

use egui::{
    epaint::ahash::HashSet, Align, Color32, Context, Layout, RichText, Sense, Ui, Widget,
    WidgetText,
};
use egui_dock::{NodeIndex, Style, Tree};
use egui_gizmo::{Gizmo, GizmoMode};
use glam::{vec4, Mat4, Vec3};

use pan_orbit_camera::OrbitCameraController;
use rose_core::light::Light;
use rose_core::transform::{Transform, TransformExt};
use rose_renderer::Renderer;

use crate::scene::{Entity, Scene};
use crate::{Combined, UiMessage};

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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Tabs {
    SceneHierarchy,
    Properties,
    RendererDebug,
    CameraControls,
    Postprocess,
    NewLight,
    Viewport,
}

impl Tabs {
    const ALL: &'static [Tabs] = &[
        Self::SceneHierarchy,
        Self::Properties,
        Self::RendererDebug,
        Self::CameraControls,
        Self::Postprocess,
        Self::NewLight,
        Self::Viewport,
    ];

    fn ui(&self, ui: &mut Ui, viewer: &mut TabViewer) {
        match self {
            Self::SceneHierarchy => self.ui_hierarchy(ui, viewer),
            Self::Properties => match *viewer.selection {
                Some(id) => self.ui_properties(ui, viewer, id),
                None => self.ui_properties_default(ui),
            },
            Self::RendererDebug => viewer.renderer.ui_debug_panel(ui),
            Self::CameraControls => viewer.camera_controller.ui_inner(ui),
            Self::Postprocess => viewer.renderer.ui_postprocessing(ui),
            Self::NewLight => self.ui_add_light(ui, viewer),
            Self::Viewport => self.ui_viewport(ui, viewer),
        }
    }

    fn title(&self) -> &'static str {
        match self {
            Self::SceneHierarchy => "Scene hierarchy",
            Self::Properties => "Properties",
            Self::RendererDebug => "Renderer debug",
            Self::CameraControls => "Camera controls",
            Self::Postprocess => "Post processing",
            Self::NewLight => "Add light",
            Self::Viewport => "Viewport",
        }
    }

    fn ui_hierarchy(&self, ui: &mut Ui, viewer: &mut TabViewer) {
        for inst in viewer.scene.instances_mut() {
            let title = if let Some(name) = &inst.name {
                RichText::new(format!("[{}] {}", inst.id(), name))
            } else {
                RichText::new(format!("[{}]", inst.id()))
            };
            ui.scope(|ui| {
                let is_selected = viewer
                    .selection
                    .map(|sel| sel == inst.id())
                    .unwrap_or(false);
                if is_selected {
                    ui.style_mut().visuals.dark_mode = !ui.style().visuals.dark_mode;
                }
                let row_response = ui
                    .collapsing(title, |ui| {
                        ui.scope(|ui| {
                            egui::Grid::new("details").num_columns(2).show(ui, |ui| {
                                ui.label("Type");
                                ui.strong(match inst.entity() {
                                    Entity::Camera(..) => "Camera",
                                    Entity::Object(..) => "Object",
                                    Entity::Light(..) => "Light",
                                });
                                ui.end_row();
                            });
                        });
                    })
                    .header_response
                    .context_menu(|ui| {
                        if let Some(name) = &mut inst.name {
                            ui.horizontal(|ui| {
                                let label = ui.label("Rename");
                                ui.text_edit_singleline(name).labelled_by(label.id);
                            });
                        }
                        if ui.button("Delete").clicked() {
                            viewer.ui_events.send(UiMessage::DeleteInstance(inst.id()));
                            if *viewer.selection == Some(inst.id()) {
                                viewer.selection.take();
                            }
                            ui.close_menu();
                        }
                    });
                if row_response.clicked() {
                    viewer.selection.replace(inst.id());
                }
            });
        }
        let size = ui.available_size();
        let (_, response) = ui.allocate_exact_size(size, Sense::click());
        let response = response.context_menu(|ui| {
            ui.weak(RichText::new("Empty").monospace());
        });
        if response.clicked() {
            viewer.selection.take();
        }
    }

    fn ui_properties(&self, ui: &mut Ui, viewer: &mut TabViewer, id: u64) {
        let Some(selected) = viewer.scene.get_mut(id) else {
            ui.label(RichText::from("Invalid ID").strong().monospace().color(Color32::RED));
            return;
        };
        ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    if let Some(name) = &mut selected.name {
                        let label = ui.label("Name");
                        ui.text_edit_singleline(name).labelled_by(label.id);
                    } else {
                        if ui.button("Add name").clicked() {
                            selected.name.replace(String::new());
                        }
                    }
                });
            });
            ui.collapsing("Transform", |ui| {
                egui::Grid::new("selected-transform")
                    .striped(true)
                    .num_columns(4)
                    .show(ui, |ui| {
                        ui.strong("Position");
                        ui.add(
                            egui::DragValue::new(&mut selected.transform.position.x)
                                .prefix("x: ")
                                .fixed_decimals(2),
                        );
                        ui.add(
                            egui::DragValue::new(&mut selected.transform.position.y)
                                .prefix("y: ")
                                .fixed_decimals(2),
                        );
                        ui.add(
                            egui::DragValue::new(&mut selected.transform.position.z)
                                .prefix("z: ")
                                .fixed_decimals(2),
                        );
                        ui.end_row();

                        ui.strong("Rotation");
                        ui.add(
                            egui::DragValue::new(&mut selected.transform.rotation.x)
                                .prefix("x: ")
                                .fixed_decimals(2),
                        );
                        ui.add(
                            egui::DragValue::new(&mut selected.transform.rotation.y)
                                .prefix("y: ")
                                .fixed_decimals(2),
                        );
                        ui.add(
                            egui::DragValue::new(&mut selected.transform.rotation.z)
                                .prefix("z: ")
                                .fixed_decimals(2),
                        );
                        ui.add(
                            egui::DragValue::new(&mut selected.transform.rotation.w)
                                .prefix("w: ")
                                .fixed_decimals(2),
                        );
                        ui.end_row();

                        ui.strong("Scale");
                        ui.add(
                            egui::DragValue::new(&mut selected.transform.scale.x)
                                .prefix("x: ")
                                .fixed_decimals(2),
                        );
                        ui.add(
                            egui::DragValue::new(&mut selected.transform.scale.y)
                                .prefix("y: ")
                                .fixed_decimals(2),
                        );
                        ui.add(
                            egui::DragValue::new(&mut selected.transform.scale.z)
                                .prefix("z: ")
                                .fixed_decimals(2),
                        );
                        ui.end_row();
                    });
            });
        });
    }
    fn ui_properties_default(&self, ui: &mut Ui) {
        ui.weak("Select an object to display its properties here");
    }
    fn ui_add_light(&self, ui: &mut Ui, viewer: &TabViewer) {
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
                ui.color_edit_button_rgb(&mut key.borrow_mut().color)
                    .labelled_by(color.id);
            });

            if ui.button("Add").clicked() {
                let new_light = key.borrow().clone();
                let color = Vec3::from_array(new_light.color);
                let light = match new_light.ty {
                    NewLightType::Ambient => Light::Ambient { color },
                    NewLightType::Point => Light::Point {
                        color,
                        position: new_light.transform.position,
                    },
                    NewLightType::Directional => Light::Directional {
                        color,
                        dir: new_light.transform.backward(),
                    },
                };
                viewer.ui_events.send(UiMessage::AddLight {
                    light,
                    respond: Some(Box::new(move |light, ui, _| {
                        ui.push(UiMessage::InstanceLight {
                            light: light.transformed(new_light.transform),
                            respond: None,
                        })
                    })),
                })
            }
        });
    }

    fn ui_viewport(&self, ui: &mut Ui, viewer: &mut TabViewer) {
        egui::Frame::none().show(ui, |ui| {
            let (ui_rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
            let rect = vec4(
                ui_rect.left(),
                ui_rect.bottom(),
                ui_rect.width(),
                ui_rect.height(),
            ) * ui.ctx().pixels_per_point();
            let (x, y, w, h) = rect.as_ivec4().into();
            viewer.renderer.set_viewport(x, y, w, h);
            let camera = viewer.renderer.camera_mut();
            if let Some(inst) = viewer
                .selection
                .as_ref()
                .copied()
                .and_then(|i| viewer.scene.get_mut(i))
            {
                let gizmo = Gizmo::new("manipulator")
                    .view_matrix(camera.transform.to_cols_array_2d())
                    .projection_matrix(camera.projection.matrix().to_cols_array_2d())
                    .model_matrix(inst.transform.matrix().to_cols_array_2d())
                    .mode(viewer.gizmo_mode);
                if let Some(response) = gizmo.interact(ui) {
                    inst.transform =
                        Transform::from_matrix(Mat4::from_cols_array_2d(&response.transform));
                }
            }
        });
    }
}

pub struct UiState {
    pub tabs: Tree<Tabs>,
    open_tabs: HashSet<Tabs>,
}

impl UiState {
    // pub fn new() -> Self {
    //     let mut tabs = Tree::new(vec![Tabs::SceneHierarchy]);
    //     tabs.split_below(NodeIndex::root(), 0.5, vec![Tabs::Properties]);
    //     Self {
    //         tabs,
    //         open_tabs: HashSet::from_iter([Tabs::SceneHierarchy, Tabs::Properties]),
    //     }
    // }

    pub fn new() -> Self {
        let mut tabs = Tree::new(vec![Tabs::Viewport]);
        let [main, left] = tabs.split_left(
            NodeIndex::root(),
            0.2,
            vec![Tabs::SceneHierarchy, Tabs::Properties],
        );
        let [main, bottom] =
            tabs.split_below(main, 0.2, vec![Tabs::Postprocess, Tabs::CameraControls]);
        let open_tabs = HashSet::from_iter(tabs.tabs().copied());
        Self { tabs, open_tabs }
    }

    pub(crate) fn show(&mut self, ctx: &Context, viewer: &mut TabViewer) {
        let style = self.dock_style(ctx.style().as_ref());
        egui_dock::DockArea::new(&mut self.tabs)
            .style(style)
            .show(ctx, viewer);
    }

    pub(crate) fn show_inner(&mut self, ui: &mut Ui, viewer: &mut TabViewer) {
        let style = self.dock_style(ui.style().as_ref());
        egui_dock::DockArea::new(&mut self.tabs)
            .style(style)
            .show_inside(ui, viewer);

        for (node, tab) in viewer.added_nodes.drain(..) {
            self.tabs.set_focused_node(node);
            self.tabs.push_to_focused_leaf(tab);
        }
    }

    pub fn ui_toolbar(&mut self, ui: &mut Ui) {
        for tab in Tabs::ALL {
            if ui
                .selectable_label(self.open_tabs.contains(tab), tab.title())
                .clicked()
            {
                if let Some(ix) = self.tabs.find_tab(tab) {
                    self.tabs.remove_tab(ix);
                    self.open_tabs.remove(tab);
                } else {
                    self.tabs.push_to_focused_leaf(*tab);
                }
                ui.close_menu();
            }
        }
    }

    fn dock_style(&self, style: &egui::Style) -> Style {
        egui_dock::StyleBuilder::from_egui(style)
            .show_close_buttons(true)
            .show_add_buttons(true)
            .show_add_popup(true)
            .build()
    }
}

pub struct TabViewer<'a> {
    pub(crate) ui_events: Combined<UiMessage>,
    pub(crate) selection: &'a mut Option<u64>,
    pub(crate) scene: &'a mut Scene,
    pub(crate) renderer: &'a mut Renderer,
    pub(crate) camera_controller: &'a mut OrbitCameraController,
    pub(crate) added_nodes: Vec<(NodeIndex, Tabs)>,
    pub(crate) gizmo_mode: GizmoMode,
}

impl<'a> egui_dock::TabViewer for TabViewer<'a> {
    type Tab = Tabs;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        tab.ui(ui, self);
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        WidgetText::from(tab.title()).strong()
    }

    fn add_popup(&mut self, ui: &mut Ui, node: NodeIndex) {
        ui.set_min_width(150.);
        for tab in Tabs::ALL {
            if ui.small_button(tab.title()).clicked() {
                self.added_nodes.push((node, *tab));
            }
        }
    }
}
