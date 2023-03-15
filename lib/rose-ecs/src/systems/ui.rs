use std::any::TypeId;
use std::collections::HashMap;
use std::f32::consts::PI;

use assets_manager::Handle;
use egui::{DragValue, Grid, RichText, Ui};
use glam::{vec3, EulerRot, Quat, Vec3};
use hecs::{CommandBuffer, Component, EntityRef};

use rose_core::transform::Transform;

use crate::NamedComponent;

pub trait ComponentUi: NamedComponent + Component {
    fn ui(&mut self, ui: &mut Ui);
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

pub struct UiSystem {
    component_ui_registry: HashMap<TypeId, DynComponentUi>,
    spawner_registry: Vec<DynInsertComponent>,
}

impl UiSystem {
    pub fn new() -> Self {
        Self {
            component_ui_registry: HashMap::new(),
            spawner_registry: vec![],
        }
    }

    pub fn register_component<C: ComponentUi>(&mut self) -> &mut Self {
        self.component_ui_registry
            .insert(TypeId::of::<C>(), &component_ui::<C>);
        self
    }

    pub fn register_spawn<C: NamedComponent + Default>(&mut self) -> &mut Self {
        self.spawner_registry.push(&insert_component::<C>);
        self
    }

    pub fn components_ui(&self, ui: &mut Ui, entity: EntityRef, cmdbuf: &mut CommandBuffer) {
        for cmp_ui in self.component_ui_registry.values() {
            cmp_ui(ui, entity, cmdbuf);
        }
    }

    pub fn spawn_component_popup(
        &self,
        ui: &mut Ui,
        entity: EntityRef,
        cmdbuf: &mut CommandBuffer,
    ) {
        for spawn_ui in &self.spawner_registry {
            spawn_ui(ui, entity, cmdbuf);
        }
    }
}

pub fn component_ui<T: ComponentUi>(ui: &mut Ui, entity: EntityRef, cmd: &mut CommandBuffer) {
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

pub type DynComponentUi =
    &'static (dyn Send + Sync + Fn(&mut Ui, EntityRef<'_>, &mut CommandBuffer));

pub fn insert_component<C: NamedComponent + Default>(
    ui: &mut Ui,
    entity: EntityRef,
    cmd: &mut CommandBuffer,
) {
    if ui.small_button(C::NAME).clicked() {
        cmd.insert_one(entity.entity(), C::default());
    }
}

pub type DynInsertComponent =
    &'static (dyn Fn(&mut Ui, EntityRef<'_>, &mut CommandBuffer) + Send + Sync);
