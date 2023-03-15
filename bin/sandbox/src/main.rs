use std::path::Path;

use egui_gizmo::GizmoMode;
use rfd::FileDialog;

use rose::ecs::load_gltf::load_gltf_scene;
use rose::prelude::*;
use violette::framebuffer::{ClearBuffer, Framebuffer};

use crate::ui::EditorUiSystem;

pub mod ui;

struct Sandbox {
    core_systems: CoreSystems,
    editor_cam_controller: PanOrbitCamera,
    pan_orbit_system: PanOrbitSystem,
    ui_system: EditorUiSystem,
    editor_scene: Option<Scene>,
    active_scene: Option<Scene>,
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
            self.core_systems.save_scene(scene)?;
        }
        Ok(())
    }

    fn stop_active_scene(&mut self) {
        self.active_scene.take();
    }

    fn do_open_scene(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let scene = self.core_systems.load_scene(path)?;
        self.editor_scene.replace(scene);
        self.active_scene.take();
        Ok(())
    }

    fn start_active_scene(&mut self) {
        self.stop_active_scene();
        if let Some(scene) = &self.editor_scene {
            match scene.reload(&mut self.core_systems.persistence) {
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
        let mut core_systems = CoreSystems::new(size)?;
        let editor_scene = std::env::args().nth(1).and_then(|file| {
            match Scene::load(&mut core_systems.persistence, file) {
                Ok(scene) => Some(scene),
                Err(err) => {
                    tracing::error!("Cannot load scene: {}", err);
                    None
                }
            }
        });

        let ui_system = EditorUiSystem::new();

        Ok(Self {
            editor_scene,
            active_scene: None,
            editor_cam_controller: PanOrbitCamera::default(),
            core_systems,
            pan_orbit_system: PanOrbitSystem::new(logical_size),
            ui_system,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>, scale_factor: f64) -> Result<()> {
        self.core_systems.resize(size)?;
        self.pan_orbit_system
            .set_window_size(size.to_logical(scale_factor));
        Ok(())
    }

    fn interact(&mut self, event: WindowEvent) -> Result<()> {
        if let Some(event) = self.core_systems.on_event(event) {
            match event {
                WindowEvent::DroppedFile(possible_env_map) => {
                    match EnvironmentMap::load(
                        possible_env_map,
                        self.core_systems.render.renderer.reload_watcher(),
                    ) {
                        Ok(env) => self.core_systems.render.renderer.set_environment(|_| env),
                        Err(err) => {
                            tracing::error!("Cannot load environment map: {}", err);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn render(&mut self, ctx: RenderContext) -> Result<()> {
        self.core_systems.begin_frame();
        if let Some(scene) = &mut self.active_scene {
            self.core_systems.manual_camera_update = false;
            scene.on_frame();
            scene.with_world_mut(|world| {
                self.pan_orbit_system
                    .on_frame(self.core_systems.input(), world)
            });
        } else if let Some(scene) = &mut self.editor_scene {
            self.core_systems.manual_camera_update = true;
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
                &mut self.core_systems.viewport_camera_mut().transform,
            );
        } else {
            Framebuffer::clear_color(Vec3::splat(0.1).extend(1.).to_array());
            Framebuffer::backbuffer().do_clear(ClearBuffer::COLOR);
        }
        self.core_systems.end_frame(
            self.active_scene.as_mut().or(self.editor_scene.as_mut()),
            ctx.dt,
        )?;
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
                    if ui.small_button("Import GLTF").clicked() {
                        let opt_file = FileDialog::new()
                            .add_filter("GLTF files", &["gltf", "glb"])
                            .pick_file();
                        if let Some(file) = opt_file {
                            match smol::block_on(load_gltf_scene(file)) {
                                Ok(scene) => {
                                    self.editor_scene.replace(scene);
                                }
                                Err(err) => {
                                    tracing::error!("Cannot import scene: {}", err);
                                }
                            }
                        }
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
                if let Some(scene) = &mut self.editor_scene {
                    ui.menu_button("Entity", |ui| {
                        if ui.small_button("Add empty").clicked() {
                            scene.with_world(|_, cmd| cmd.spawn(()));
                            ui.close_menu();
                        }
                        ui.menu_button("Templates", |ui| {
                            if ui.small_button("Mesh").clicked() {
                                scene.with_world(|_world, cmd| {
                                    let cache = scene.asset_cache().as_any_cache();
                                    let mesh = self.core_systems.render.primitive_cube(cache);
                                    let material =
                                        self.core_systems.render.default_material_handle(cache);
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
                        if ui.small_button("Insert nested ...").clicked() {
                            let opt_file = FileDialog::new()
                                .add_filter("Supported", &["scene", "toml", "gltf", "glb"])
                                .add_filter("Scenes", &["scene"])
                                .add_filter("TOML files", &["toml"])
                                .add_filter("GLTF scenes", &["gltf", "glb"])
                                .pick_file();
                            if let Some(file) = opt_file {
                                let nested =
                                    match file.extension().unwrap().to_string_lossy().as_ref() {
                                        "scene" | "toml" => Scene::load(
                                            &mut self.core_systems.persistence,
                                            file.as_path(),
                                        ),
                                        "gltf" | "glb" => {
                                            smol::block_on(load_gltf_scene(file.as_path()))
                                        }
                                        _ => unreachable!(),
                                    };
                                match nested.and_then(|nested| scene.add_nested(nested)) {
                                    Ok(()) => {}
                                    Err(err) => {
                                        tracing::error!("Cannot add nested scene: {}", err);
                                    }
                                }
                            }
                        }
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
                ui.separator();
                if self.active_scene.is_some() {
                    if ui.small_button("Stop scene").clicked() {
                        self.stop_active_scene();
                    }
                } else {
                    if ui.small_button("Play").clicked() {
                        self.start_active_scene();
                    }
                }
            });
        });
        // egui::Window::new("Environment")
        //     .show(ctx.egui, |ui| {
        //         let env = self.render_system.environment_mut();
        //         env.params.ui(ui);
        //     });
        self.ui_system
            .on_ui(ctx.egui, self.editor_scene.as_ref(), &mut self.core_systems);
    }
}

fn main() -> Result<()> {
    run::<Sandbox>("Sandbox")
}
