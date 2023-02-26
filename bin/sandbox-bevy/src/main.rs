use std::f32::consts::{PI, TAU};

use bevy::{
    app::App,
    input::mouse::{MouseMotion, MouseWheel},
    log::LogPlugin,
    prelude::*,
    DefaultPlugins,
};
use dolly::prelude::*;
use glam::{vec2, Vec3};
use rand::Rng;
use tracing::Level;

use plugins::{MeshBundle, RendererThread};
use rose_core::{
    material::{Material, Vertex},
    mesh::MeshBuilder,
};
use violette::texture::Texture;

use crate::plugins::{ActiveCamera, CameraBundle, DollyCamera, Interface, ShellPlugins};

mod plugins;

#[derive(Component)]
struct MainSphere;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: String::from("violette=debug,gl=trace,rose_core=debug,rose_renderer=debug"),
            level: Level::INFO,
        }))
        .add_plugins(ShellPlugins)
        .add_startup_system(spawn_camera)
        .add_startup_system_set_to_stage(RendererThread, SystemSet::new().with_system(add_objects))
        .add_system(rotate_sphere)
        .add_system(dolly_update)
        .run();
}

fn add_objects(mut commands: Commands) {
    let mesh = MeshBuilder::new(Vertex::new).uv_sphere(1., 32, 24).unwrap();
    let material = Material::create(
        Texture::load_rgb32f("assets/textures/moon_color.png").unwrap(),
        Some(Texture::load_rgb32f("assets/textures/moon_normal.png").unwrap()),
        // Texture::load_rg32f("assets/textures/square_floor_rough_metal.jpg").unwrap(),
        [0.5, 0.],
    )
    .unwrap();

    let mesh = crate::plugins::Mesh::from(mesh);
    let material = crate::plugins::Material::from(material);
    commands.spawn(MeshBundle {
        mesh,
        material,
        ..Default::default()
    });
    commands.spawn(Interface::new(|ctx| {
        egui::Window::new("Test").show(ctx, |ui| {
            ui.label("Test label");
        });
    }));
}

#[cfg(never)]
fn add_objects(mut commands: Commands) {
    let mesh = MeshBuilder::new(Vertex::new)
        .uv_sphere(0.3, 32, 24)
        .unwrap();
    let material = Material::create(
        Texture::load_rgb32f("assets/textures/square_floor_diffuse.jpg").unwrap(),
        Some(Texture::load_rgb32f("assets/textures/square_floor_normal.png").unwrap()),
        Texture::load_rg32f("assets/textures/square_floor_rough_metal.jpg").unwrap(),
    )
    .unwrap();

    let mesh = crate::plugins::Mesh::from(mesh);
    let material = crate::plugins::Material::from(material);
    for y in -10..10 {
        for x in -10..10 {
            commands
                .spawn(MeshBundle {
                    mesh: mesh.clone(),
                    material: material.clone(),
                    transform: Transform::from_translation(vec3(x as _, 0., y as _)),
                    ..Default::default()
                })
                .insert(MainSphere);
        }
    }

    let mut rng = thread_rng();
    for y in -4..4 {
        for x in -4..4 {
            commands.spawn(LightBundle {
                transform: Transform::from_translation(vec3(x as _, 1., y as _) * 2.5),
                light: Light {
                    kind: LightKind::Point,
                    color: rng.gen::<Vec3>().normalize(),
                    power: rng.gen_range(1.0..=10.),
                },
                ..Default::default()
            });
        }
    }
    commands.spawn(LightBundle {
        transform: Transform::from_translation(vec3(1., 0.1, 0.)).looking_at(Vec3::ZERO, Vec3::Y),
        light: Light {
            kind: LightKind::Directional,
            color: Vec3::ONE,
            power: 10.,
        },
        ..Default::default()
    });
    commands.spawn(LightBundle {
        light: Light {
            kind: LightKind::Ambient,
            color: vec3(0.5, 0.8, 1.0),
            power: 1.,
        },
        ..Default::default()
    });
}

fn rotate_sphere(time: Res<Time>, mut query: Query<&mut Transform, With<MainSphere>>) {
    let t = time.elapsed_seconds();
    for mut transform in &mut query {
        transform.rotation = Quat::from_rotation_y(t / 10.);
    }
}

fn dolly_update(
    windows: Res<Windows>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    input_mouse: Res<Input<MouseButton>>,
    mut query: Query<&mut DollyCamera, With<ActiveCamera>>,
) {
    let winsize = get_primary_window_size(&windows);
    // let aspect_ratio = winsize.x / winsize.y;

    let delta = ev_motion.iter().map(|motion| motion.delta).sum::<Vec2>() / winsize * vec2(TAU, PI);
    let scroll = ev_scroll.iter().map(|ev| ev.y).sum::<f32>();

    for mut dolly in &mut query {
        // if input_mouse.pressed(MouseButton::Left) {
        //     let yaw_pitch = dolly.0.driver_mut::<YawPitch>();
        //     yaw_pitch.rotate_yaw_pitch(delta.x, delta.y);
        // }
        if input_mouse.pressed(MouseButton::Right) {
            let tr = dolly.0.final_transform;
            let arm = dolly.0.driver_mut::<Arm>();
            let offset = tr.right() * delta.x - tr.up() * delta.y;
            arm.offset += offset;
        }
        //
        // let arm = dolly.0.driver_mut::<Arm>();
        // arm.offset *= scroll * arm.offset.length() * 0.2;
        // arm.offset = arm.offset.clamp_length_min(0.05);
    }
}

/// Tags an entity as capable of panning and orbiting.
#[cfg(never)]
#[derive(Component)]
struct PanOrbitCamera {
    /// The "focus point" to orbit around. It is automatically updated when panning the camera
    pub focus: Vec3,
    pub radius: f32,
    pub upside_down: bool,
}

#[cfg(never)]
impl Default for PanOrbitCamera {
    fn default() -> Self {
        PanOrbitCamera {
            focus: Vec3::ZERO,
            radius: 5.0,
            upside_down: false,
        }
    }
}

#[cfg(never)]
fn pan_orbit_camera(
    windows: Res<Windows>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    input_mouse: Res<Input<MouseButton>>,
    mut query: Query<(&mut PanOrbitCamera, &mut Transform, &Camera)>,
) {
    let winsize = get_primary_window_size(&windows);
    let aspect_ratio = winsize.x / winsize.y;

    let delta = ev_motion.iter().map(|motion| motion.delta).sum::<Vec2>() / winsize * vec2(TAU, PI);
    let scroll = ev_scroll.iter().map(|ev| ev.y).sum::<f32>();

    let quat = if input_mouse.pressed(MouseButton::Left) {
        Quat::from_euler(EulerRot::ZXY, 0., -delta.y, -delta.x)
    } else {
        Quat::IDENTITY
    };

    for (mut controller, mut transform, _camera) in &mut query {
        transform.rotate(quat);
        controller.radius -= scroll * controller.radius * 0.2;
        // dont allow zoom to reach zero or you get stuck
        controller.radius = f32::max(controller.radius, 0.05);
        if input_mouse.pressed(MouseButton::Right) {
            let disp = delta.x * transform.right() + delta.y * aspect_ratio * transform.down();
            let radius = controller.radius;
            controller.focus += disp * radius;
        }
        transform.translation = controller.focus + transform.left() * controller.radius;
        // transform.look_at(controller.focus, Vec3::Y);
    }
}

/// Pan the camera with middle mouse click, zoom with scroll wheel, orbit with right mouse click.
#[cfg(never)]
fn pan_orbit_camera(
    windows: Res<Windows>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    input_mouse: Res<Input<MouseButton>>,
    mut query: Query<(&mut PanOrbitCamera, &mut Transform, &Camera)>,
) {
    // change input mapping for orbit and panning here
    let orbit_button = MouseButton::Left;
    let pan_button = MouseButton::Right;

    let mut pan = Vec2::ZERO;
    let mut rotation_move = Vec2::ZERO;
    let mut scroll = 0.0;
    let mut orbit_button_changed = false;

    if input_mouse.pressed(orbit_button) {
        for ev in ev_motion.iter() {
            rotation_move += ev.delta;
        }
    } else if input_mouse.pressed(pan_button) {
        // Pan only if we're not rotating at the moment
        for ev in ev_motion.iter() {
            pan += ev.delta;
        }
    }
    for ev in ev_scroll.iter() {
        scroll += ev.y;
    }
    if input_mouse.just_released(orbit_button) || input_mouse.just_pressed(orbit_button) {
        orbit_button_changed = true;
    }

    for (mut pan_orbit, mut transform, projection) in query.iter_mut() {
        if orbit_button_changed {
            // only check for upside down when orbiting started or ended this frame
            // if the camera is "upside" down, panning horizontally would be inverted, so invert the input to make it correct
            let up = transform.rotation * Vec3::Y;
            pan_orbit.upside_down = up.y <= 0.0;
        }

        let mut any = false;
        if rotation_move.length_squared() > 0.0 {
            any = true;
            let window = get_primary_window_size(&windows);
            let delta_x = {
                let delta = rotation_move.x / window.x * std::f32::consts::PI * 2.0;
                if pan_orbit.upside_down {
                    -delta
                } else {
                    delta
                }
            };
            let delta_y = rotation_move.y / window.y * std::f32::consts::PI;
            let yaw = Quat::from_rotation_y(-delta_x);
            let pitch = Quat::from_rotation_x(-delta_y);
            transform.rotation = yaw * transform.rotation; // rotate around global y axis
            transform.rotation = transform.rotation * pitch; // rotate around local x axis
        } else if pan.length_squared() > 0.0 {
            any = true;
            // make panning distance independent of resolution and FOV,
            let window = get_primary_window_size(&windows);
            let aspect_ratio = window.x / window.y;
            pan *= Vec2::new(projection.fovy * aspect_ratio, projection.fovy) / window;
            // translate by local axes
            let right = transform.rotation * Vec3::X * -pan.x;
            let up = transform.rotation * Vec3::Y * pan.y;
            // make panning proportional to distance away from focus point
            let translation = (right + up) * pan_orbit.radius;
            pan_orbit.focus += translation;
        } else if scroll.abs() > 0.0 {
            any = true;
            pan_orbit.radius -= scroll * pan_orbit.radius * 0.2;
            // dont allow zoom to reach zero or you get stuck
            pan_orbit.radius = f32::max(pan_orbit.radius, 0.05);
        }

        if any {
            // emulating parent/child to make the yaw/y-axis rotation behave like a turntable
            // parent = x and y rotation
            // child = z-offset
            let rot_matrix = Mat3::from_quat(transform.rotation);
            transform.translation =
                pan_orbit.focus - rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, pan_orbit.radius));
        }
    }

    // consume any remaining events, so they don't pile up if we don't need them
    // (and also to avoid Bevy warning us about not checking events every frame update)
    ev_motion.clear();
}

fn spawn_camera(mut commands: Commands) {
    // let translation = Vec3::new(-2.0, 2.5, 5.0);
    // let radius = translation.length();
    commands.spawn((
        CameraBundle::default(),
        // PanOrbitCamera {
        //     radius,
        //     ..Default::default()
        // },
        DollyCamera(
            CameraRig::builder()
                .with(YawPitch::new().yaw_degrees(45.0).pitch_degrees(-30.0))
                .with(Smooth::new_rotation(1.5))
                .with(Arm::new(Vec3::Z * 8.0))
                .build(),
        ),
        ActiveCamera,
    ));
}

fn get_primary_window_size(windows: &Res<Windows>) -> Vec2 {
    let window = windows.get_primary().unwrap();
    let window = Vec2::new(window.width() as f32, window.height() as f32);
    window
}

#[cfg(never)]
fn export_entites_components(world: &mut World) {
    let mut csv = csv::WriterBuilder::new()
        .delimiter(b',')
        .double_quote(true)
        .has_headers(true)
        .from_path("entities.csv")
        .unwrap();
    csv.write_record([
        "Entity generation",
        "Entity index",
        "Component id",
        "Component name",
        "Component type",
    ])
    .unwrap();
    for entity in world.query::<Entity>().iter(world) {
        for info in world.inspect_entity(entity) {
            info!(
                "Looking at entity {:?}, component {:?} {}",
                entity,
                info.id(),
                info.name()
            );
            csv.write_record(&[
                entity.generation().to_string(),
                entity.index().to_string(),
                format!("{:?}", info.id()),
                info.name().to_string(),
                format!("{:?}", info.type_id()),
            ])
            .unwrap();
        }
    }
}
