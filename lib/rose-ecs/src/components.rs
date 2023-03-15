use std::{
    f32::consts::PI,
    hash::{Hash, Hasher},
    ops::Range,
};

use assets_manager::SharedString;
use egui::{DragValue, Grid, Ui};
use glam::{Vec2, Vec3};
use hecs::Bundle;
use serde::{Deserialize, Serialize};

use rose_core::{camera::Projection, transform::Transform};

#[cfg(feature = "ui")]
use crate::systems::ComponentUi;
use crate::NamedComponent;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Active;

#[cfg(feature = "ui")]
impl ComponentUi for Active {
    fn ui(&mut self, ui: &mut Ui) {
        ui.weak("No associated component data");
    }
}

impl NamedComponent for Active {
    const NAME: &'static str = "Active";
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Inactive;

#[cfg(feature = "ui")]
impl ComponentUi for Inactive {
    fn ui(&mut self, ui: &mut Ui) {
        ui.weak("No associated component data");
    }
}

impl NamedComponent for Inactive {
    const NAME: &'static str = "Inactive";
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct CameraParams {
    pub fovy: f32,
    pub zrange: Range<f32>,
}

impl From<Projection> for CameraParams {
    fn from(value: Projection) -> Self {
        Self {
            fovy: value.fovy,
            zrange: value.zrange,
        }
    }
}

#[cfg(feature = "ui")]
impl ComponentUi for CameraParams {
    fn ui(&mut self, ui: &mut Ui) {
        Grid::new("camera-params").num_columns(2).show(ui, |ui| {
            let fov_label = ui.label("Vert. FOV").id;
            self.fovy *= 180. / PI;
            ui.add(DragValue::new(&mut self.fovy).suffix(" °"))
                .labelled_by(fov_label);
            self.fovy *= PI / 180.;
            ui.end_row();

            let zrange_label = ui.label("Z Range").id;
            ui.horizontal(|ui| {
                ui.add(
                    DragValue::new(&mut self.zrange.start)
                        .prefix("start:")
                        .suffix(" m"),
                );
                ui.add(
                    DragValue::new(&mut self.zrange.end)
                        .prefix("end:")
                        .suffix(" m"),
                );
            })
            .response
            .labelled_by(zrange_label);
        });
    }
}

impl NamedComponent for CameraParams {
    const NAME: &'static str = "Camera Parameters";
}

impl Default for CameraParams {
    fn default() -> Self {
        Self {
            fovy: 45f32,
            zrange: 1e-3..1e3,
        }
    }
}

#[derive(Debug, Clone, Default, Bundle)]
pub struct CameraBundle {
    pub transform: Transform,
    pub params: CameraParams,
    pub active: Active,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub struct PanOrbitCamera {
    pub target_rotation: Vec2,
    pub radius: f32,
    pub focus: Vec3,
}

#[cfg(feature = "ui")]
impl ComponentUi for PanOrbitCamera {
    fn ui(&mut self, ui: &mut Ui) {
        Grid::new("pan-orbit-camera").num_columns(2).show(ui, |ui| {
            let rot_label = ui.label("Tgt. rotation").id;
            ui.horizontal(|ui| {
                self.target_rotation *= 180. / PI;
                ui.add(
                    DragValue::new(&mut self.target_rotation.y)
                        .prefix("Lat:")
                        .suffix(" °"),
                );
                ui.add(
                    DragValue::new(&mut self.target_rotation.x)
                        .prefix("Lon:")
                        .suffix(" °"),
                );
                self.target_rotation *= 180. / PI;
            })
            .response
            .labelled_by(rot_label);
            ui.end_row();

            let radius_label = ui.label("Radius").id;
            ui.add(DragValue::new(&mut self.radius).suffix(" m"))
                .labelled_by(radius_label);
            ui.end_row();

            let focus_label = ui.label("Focus").id;
            ui.horizontal(|ui| {
                ui.add(DragValue::new(&mut self.focus.x).prefix("X:").suffix(" m"));
                ui.add(DragValue::new(&mut self.focus.y).prefix("Y:").suffix(" m"));
                ui.add(DragValue::new(&mut self.focus.z).prefix("Z:").suffix(" m"));
            })
            .response
            .labelled_by(focus_label);
            // ui.end_row();
        });
    }
}

impl NamedComponent for PanOrbitCamera {
    const NAME: &'static str = "Pan/Orbit Camera";
}

impl Default for PanOrbitCamera {
    fn default() -> Self {
        Self {
            target_rotation: Vec2::ZERO,
            radius: 5.,
            focus: Vec3::ZERO,
        }
    }
}

#[derive(Debug, Default, Bundle)]
pub struct PanOrbitCameraBundle {
    pub transform: Transform,
    pub params: CameraParams,
    pub pan_orbit: PanOrbitCamera,
    pub active: Active,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub enum LightKind {
    Ambient,
    Point,
    Directional,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Light {
    pub kind: LightKind,
    pub color: Vec3,
    pub power: f32,
}

#[cfg(feature = "ui")]
impl ComponentUi for Light {
    fn ui(&mut self, ui: &mut Ui) {
        Grid::new("component-light").num_columns(2).show(ui, |ui| {
            let kind_label = ui.label("Kind").id;
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.kind, LightKind::Point, "Point");
                ui.radio_value(&mut self.kind, LightKind::Directional, "Directional");
                ui.radio_value(&mut self.kind, LightKind::Ambient, "Ambient");
            })
            .response
            .labelled_by(kind_label);
            ui.end_row();

            let color_label = ui.label("Color").id;
            ui.color_edit_button_rgb(self.color.as_mut())
                .labelled_by(color_label);
            ui.end_row();

            let power_label = ui.label("Power").id;
            ui.add(DragValue::new(&mut self.power).suffix(" W"))
                .labelled_by(power_label);
            // ui.end_row();
        });
    }
}

impl NamedComponent for Light {
    const NAME: &'static str = "Light";
}

impl Hash for Light {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        for f in self.color.to_array() {
            f.to_bits().hash(state);
        }
        self.power.to_bits().hash(state);
    }
}

impl Default for Light {
    fn default() -> Self {
        Self {
            kind: LightKind::Point,
            color: Vec3::ONE,
            power: 1.,
        }
    }
}

#[derive(Debug, Default, Bundle)]
pub struct LightBundle {
    pub light: Light,
    pub transform: Transform,
    pub active: Active,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SceneId(pub SharedString);

#[cfg(feature = "ui")]
impl ComponentUi for SceneId {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("Scene ID marker for tracking nested scenes.");
        ui.monospace(self.0.as_str());
    }
}

impl NamedComponent for SceneId {
    const NAME: &'static str = "Scene ID";
}
