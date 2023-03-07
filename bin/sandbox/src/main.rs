use std::path::Path;

use assets_manager::Handle;
use egui_gizmo::GizmoMode;
use eyre::Result;
use glam::{Vec2, Vec3};
use hecs::EntityBuilder;
use rfd::FileDialog;

use rose_core::transform::Transform;
use rose_core::utils::thread_guard::ThreadGuard;
use rose_platform::{
    Application, events::WindowEvent, LogicalSize, PhysicalSize, RenderContext, UiContext,
    WindowBuilder,
};
use violette::framebuffer::{ClearBuffer, Framebuffer};

use crate::{
    assets::{
        material::Material,
        mesh::MeshAsset,
        object::ObjectBundle,
    },
    components::{
        Active,
        CameraParams,
        Inactive,
        Light,
        LightBundle,
        PanOrbitCamera,
        SceneId,
    },
    scene::Scene,
    systems::{
        camera::PanOrbitSystem,
        input::InputSystem,
        persistence::PersistenceSystem,
        render::RenderSystem,
        ui::UiSystem,
    },
};

mod assets;
pub mod components;
mod scene;
mod systems;

struct Sandbox {
    editor_scene: Option<Scene>,
    active_scene: Option<Scene>,
    editor_cam_controller: PanOrbitCamera,
    input_system: InputSystem,
    render_system: RenderSystem,
    pan_orbit_system: PanOrbitSystem,
    ui_system: UiSystem,
    persistence_system: ThreadGuard<PersistenceSystem>,
}

impl Sandbox {
    fn new_scene(&mut self) {
        self.active_scene.take();
        self.editor_scene.take();
        if let Some(folder) = FileDialog::new().pick_folder() {
            match Scene::new(folder) {
                Ok(scene) => {
                    self.editor_scene.replace(scene);
                }
                Err(err) => tracing::error!("Cannot create new scene: {}", err),
            }
        }
    }

    fn open_scene(&mut self) -> Result<()> {
        let file = FileDialog::new()
            .add_filter("Scenes", &["scene"])
            .add_filter("TOML files", &["toml"])
            .set_directory(std::env::current_dir().unwrap())
            .pick_file();
        if let Some(file) = file {
            self.do_open_scene(file)?;
        }
        Ok(())
    }

    fn save_scene(&mut self) -> Result<()> {
        let file = FileDialog::new()
            .add_filter("Scenes", &["scene"])
            .set_directory(std::env::current_dir().unwrap())
            .save_file();
        if let Some(file) = file {
            self.save_scene_as(file)?;
        }
        Ok(())
    }

    fn save_scene_as(&mut self, path: impl AsRef<Path>) -> Result<()> {
        if let Some(scene) = &mut self.editor_scene {
            scene.set_path(path);
            scene.save(&mut self.persistence_system)?;
        }
        Ok(())
    }

    fn stop_active_scene(&mut self) {
        self.active_scene.take();
    }

    fn do_open_scene(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let scene = Scene::load(&mut self.persistence_system, path)?;
        self.editor_scene.replace(scene);
        self.active_scene.take();
        Ok(())
    }

    fn start_active_scene(&mut self) {
        self.stop_active_scene();
        if let Some(scene) = &self.editor_scene {
            match scene.reload(&mut self.persistence_system) {
                Ok(scene) => {
                    self.active_scene.replace(scene);
                }
                Err(err) => {
                    tracing::error!("Cannot activate scene: {}", err);
                }
            }
        }
    }
}

impl Application for Sandbox {
    fn window_features(wb: WindowBuilder) -> WindowBuilder {
        wb.with_inner_size(LogicalSize::new(1600, 900))
    }

    fn new(size: PhysicalSize<f32>, scale_factor: f64) -> Result<Self> {
        let logical_size = size.to_logical(scale_factor);
        let size = Vec2::from_array(size.into()).as_uvec2();
        let mut render_system = RenderSystem::new(size)?;
        render_system.clear_color = Vec3::splat(0.1);

        let mut persistence = PersistenceSystem::new();
        persistence
            .register_component::<String>()
            .register_component::<Active>()
            .register_component::<Inactive>()
            .register_component::<Transform>()
            .register_component::<CameraParams>()
            .register_component::<PanOrbitCamera>()
            .register_component::<Light>()
            .register_asset::<MeshAsset>()
            .register_asset::<Material>();

        let editor_scene =
            std::env::args().nth(1)
                .and_then(|file| match Scene::load(&mut persistence, file) {
                    Ok(scene) => Some(scene),
                    Err(err) => {
                        tracing::error!("Cannot load scene: {}", err);
                        None
                    }
                });

        let ui_system = UiSystem::new()
            .register_component_ui::<Transform>()
            .register_component_ui::<Active>()
            .register_component_ui::<Inactive>()
            .register_component_ui::<CameraParams>()
            .register_component_ui::<PanOrbitCamera>()
            .register_component_ui::<Handle<'static, MeshAsset>>()
            .register_component_ui::<Handle<'static, Material>>()
            .register_component_ui::<Light>()
            .register_component_ui::<SceneId>()
            .register_spawn_component::<Transform>()
            .register_spawn_component::<Active>()
            .register_spawn_component::<Inactive>()
            .register_spawn_component::<CameraParams>()
            .register_spawn_component::<PanOrbitCamera>()
            .register_spawn_component::<Light>();

        Ok(Self {
            editor_scene,
            active_scene: None,
            editor_cam_controller: PanOrbitCamera::default(),
            input_system: InputSystem::default(),
            render_system,
            pan_orbit_system: PanOrbitSystem::new(logical_size),
            ui_system,
            persistence_system: ThreadGuard::new(persistence),
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>, scale_factor: f64) -> Result<()> {
        self.render_system.resize(size)?;
        self.pan_orbit_system
            .set_window_size(size.to_logical(scale_factor));
        Ok(())
    }

    fn interact(&mut self, event: WindowEvent) -> Result<()> {
        self.input_system.on_event(event);
        Ok(())
    }

    fn render(&mut self, ctx: RenderContext) -> Result<()> {
        self.input_system.on_frame();
        if let Some(scene) = &mut self.active_scene {
            scene.on_frame();
            scene.with_world_mut(|world| {
                self.pan_orbit_system
                    .on_frame(&self.input_system.input, world)
            });
            scene.with_world(|world, _| {
                self.render_system.update_from_active_camera(world);
                self.render_system.on_frame(ctx.dt, world)
            })?;
            scene.flush_commands();
        } else if let Some(scene) = &mut self.editor_scene {
            scene.on_frame();
            let win_size = ctx
                .window
                .inner_size()
                .to_logical::<f32>(ctx.window.scale_factor());
            let win_size = win_size.width.min(win_size.height);
            self.pan_orbit_system.frame_one(
                self.ui_system.last_state.mouse_delta / win_size,
                self.ui_system.last_state.mouse_scroll * ctx.dt.as_secs_f32() * 20.,
                self.ui_system.last_state.mouse_buttons,
                &mut self.editor_cam_controller,
                &mut self.render_system.camera.transform,
            );
            scene.with_world(|world, _| self.render_system.on_frame(ctx.dt, world))?;
            scene.flush_commands();
        } else {
            Framebuffer::clear_color(Vec3::splat(0.1).extend(1.).to_array());
            Framebuffer::backbuffer().do_clear(ClearBuffer::COLOR);
        }
        Ok(())
    }

    fn ui(&mut self, ctx: UiContext) {
        egui::TopBottomPanel::top("menu").show(ctx.egui, |ui| {
            ui.horizontal(|ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
                ui.separator();
                ui.menu_button("File", |ui| {
                    if ui.small_button("New").clicked() {
                        self.new_scene();
                        ui.close_menu();
                    }
                    if ui.small_button("Open...").clicked() {
                        self.open_scene().unwrap();
                        ui.close_menu();
                    }
                    if let Some(scene_path) =
                        self.editor_scene.as_ref().map(|s| s.path().to_path_buf())
                    {
                        if ui.small_button("Save").clicked() {
                            self.save_scene_as(scene_path).unwrap();
                            ui.close_menu();
                        }
                    } else {
                        ui.weak("Save");
                    }
                    if self.editor_scene.is_some() {
                        if ui.small_button("Save as...").clicked() {
                            self.save_scene().unwrap();
                            ui.close_menu();
                        }
                    } else {
                        ui.weak("Save as ...");
                    }
                });
                if let Some(scene) = &self.editor_scene {
                    ui.menu_button("Entity", |ui| {
                        if ui.small_button("Add empty").clicked() {
                            scene.with_world(|_, cmd| cmd.spawn(()));
                            ui.close_menu();
                        }
                        ui.menu_button("Templates", |ui| {
                            if ui.small_button("Mesh").clicked() {
                                scene.with_world(|_world, cmd| {
                                    let mesh =
                                        self.render_system.primitive_cube(scene.asset_cache());
                                    let material = self
                                        .render_system
                                        .default_material_handle(scene.asset_cache());
                                    cmd.spawn(
                                        EntityBuilder::new()
                                            .add(String::from("Cube"))
                                            .add_bundle(ObjectBundle {
                                                mesh,
                                                material,
                                                transform: Transform::default(),
                                                active: Active,
                                            })
                                            .build(),
                                    );
                                    // cmd.spawn((
                                    //     String::from("Cube"),
                                    //     ObjectBundle {
                                    //         mesh,
                                    //         material,
                                    //         transform: Default::default(),
                                    //     },
                                    // ));
                                });
                                ui.close_menu();
                            }
                            if ui.small_button("Point light").clicked() {
                                scene.with_world(|_, cmd| {
                                    cmd.spawn(
                                        EntityBuilder::new()
                                            .add(String::from("Point light"))
                                            .add_bundle(LightBundle::default())
                                            .build(),
                                    );
                                });
                                ui.close_menu();
                            }
                        });
                    });
                } else {
                    ui.weak("Entity");
                }
                ui.separator();
                ui.radio_value(
                    &mut self.ui_system.gizmo_mode,
                    GizmoMode::Translate,
                    "Translate",
                );
                ui.radio_value(&mut self.ui_system.gizmo_mode, GizmoMode::Rotate, "Rotate");
                ui.radio_value(&mut self.ui_system.gizmo_mode, GizmoMode::Scale, "Scale");
            });
        });
        // egui::Window::new("Environment")
        //     .show(ctx.egui, |ui| {
        //         let env = self.render_system.environment_mut();
        //         env.params.ui(ui);
        //     });
        self.ui_system.on_ui(
            ctx.egui,
            self.editor_scene.as_ref(),
            &mut self.render_system,
        );
    }
}

fn main() -> Result<()> {
    rose_platform::run::<Sandbox>("Sandbox")
}
