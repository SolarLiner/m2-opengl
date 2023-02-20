use egui::{Color32, Ui};
use glam::{EulerRot, Quat, Vec3};
use num_traits::FromPrimitive;
use num_traits::real::Real;

use crate::scene::{Component, MaterialRef, MeshRef, Named, Scene};

#[derive(Debug, Copy, Clone)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl Into<rose_core::transform::Transform> for Transform {
    fn into(self) -> rose_core::transform::Transform {
        rose_core::transform::Transform {
            position: self.position,
            rotation: Quat::from_euler(
                EulerRot::ZXY,
                self.rotation.z,
                self.rotation.x,
                self.rotation.y,
            ),
            scale: self.scale,
        }
    }
}

impl Component for Transform {
    fn ui(&mut self, ui: &mut Ui, scene: &mut Scene) {
        ui.columns(4, |ui| {
            ui[0].label("Position");
            ui[1].add(egui::DragValue::new(&mut self.position.x).prefix("X "));
            ui[2].add(egui::DragValue::new(&mut self.position.y).prefix("Y "));
            ui[3].add(egui::DragValue::new(&mut self.position.z).prefix("Z "));
        });
        ui.columns(4, |ui| {
            ui[0].label("Rotation");
            ui[1].add(
                egui::DragValue::new(&mut self.position.x)
                    .prefix("X ")
                    .suffix("°")
                    .custom_formatter(|f, _| f.to_degrees().to_string())
                    .custom_parser(|s| s.parse().ok().map(|f| f.to_radians())),
            );
            ui[2].add(
                egui::DragValue::new(&mut self.position.y)
                    .prefix("Y ")
                    .suffix("°")
                    .custom_formatter(|f, _| f.to_degrees().to_string())
                    .custom_parser(|s| s.parse().ok().map(|f| f.to_radians())),
            );
            ui[3].add(
                egui::DragValue::new(&mut self.position.z)
                    .prefix("Z ")
                    .suffix("°")
                    .custom_formatter(|f, _| f.to_degrees().to_string())
                    .custom_parser(|s| s.parse().ok().map(|f| f.to_radians())),
            );
        });
        ui.columns(4, |ui| {
            ui[0].label("Scale");
            ui[1].add(
                egui::DragValue::new(&mut self.position.x)
                    .prefix("X ")
                    .suffix(" %")
                    .custom_formatter(|v, _| (v * 100.).to_string())
                    .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.)),
            );
            ui[2].add(
                egui::DragValue::new(&mut self.position.y)
                    .prefix("Y ")
                    .suffix(" %")
                    .custom_formatter(|v, _| (v * 100.).to_string())
                    .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.)),
            );
            ui[3].add(
                egui::DragValue::new(&mut self.position.z)
                    .prefix("Z ")
                    .suffix(" %")
                    .custom_formatter(|v, _| (v * 100.).to_string())
                    .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.)),
            );
        });
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, num_derive::FromPrimitive)]
#[repr(usize)]
pub enum LightKind {
    Ambient,
    Point,
    Direction,
}

impl ToString for LightKind {
    fn to_string(&self) -> String {
        match self {
            LightKind::Ambient => "Ambient",
            LightKind::Point => "Point",
            LightKind::Direction => "Directional",
        }.to_string()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Light {
    kind: LightKind,
    color: [f32; 3],
    power: f32,
}

impl Component for Light {
    fn on_create(&mut self, scene: &mut Scene) -> eyre::Result<()> {
        scene.need_relight = true;
        Ok(())
    }
    
    fn ui(&mut self, ui: &mut Ui, scene: &mut Scene) {
        ui.columns(2, |ui| {
            ui[0].label("Kind");
            egui::ComboBox::new("light-kind", "Kind")
                .show_index(&mut ui[1], (&mut self.kind) as _, 3, |ix| LightKind::from_usize(ix).unwrap().to_string());
        });
        ui.columns(2, |ui| {
            ui[0].label("Color");
            ui[1].color_edit_button_rgb(&mut self.color)
        });
        ui.columns(2, |ui| {
            ui[0].label("Power");
            ui[1].add(egui::DragValue::new(&mut self.power).suffix(" W"));
        });
    }
}

impl Light {
    fn into_light(self, transform: rose_core::transform::Transform) -> rose_core::light::Light {
        let color = Vec3::from_array(self.color) * self.power;
        match self.kind {
            LightKind::Ambient => rose_core::light::Light::Ambient { color },
            LightKind::Direction => rose_core::light::Light::Directional {
                color,
                dir: transform.forward(),
            },
            LightKind::Point => rose_core::light::Light::Point {
                color,
                position: transform.position,
            },
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct MeshRender {
    pub mesh: MeshRef,
    pub material: MaterialRef,
}

impl Component for MeshRender {
    fn ui(&mut self, ui: &mut Ui, scene: &mut Scene) {
        ui.columns(2, |ui| {
            ui[0].label("Using mesh");
            ui[1].strong(&scene[self.mesh].name);
        });
        ui.columns(2, |ui| {
            ui[0].label("Using material");
            ui[1].strong(&scene[self.material].name);
        });
    }
}