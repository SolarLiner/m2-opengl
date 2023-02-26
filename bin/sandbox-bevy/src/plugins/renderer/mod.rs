use std::{
    collections::hash_map::DefaultHasher,
    ffi::CString,
    hash::{Hash, Hasher},
    ops,
    sync::{Arc, Weak},
};
use std::ops::Range;

use bevy::{prelude::*, window::WindowResized};
use glam::{uvec2, vec2, vec3};
use glutin::{
    prelude::*,
};
use once_cell::sync::Lazy;
use context::OpenGlContext;

use rose_core::{light::GpuLight, material::Vertex, mesh::MeshBuilder, transform::TransformExt};
use rose_renderer::Renderer;
use surface::RenderTargetSurface;
use crate::plugins::renderer::display::OpenGlDisplay;

mod display;
mod config;
mod surface;
mod context;

#[derive(Debug, Resource)]
pub struct RoseRenderer(Renderer);

impl ops::Deref for RoseRenderer {
    type Target = Renderer;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for RoseRenderer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromWorld for RoseRenderer {
    fn from_world(world: &mut World) -> Self {
        let window = world.resource::<Windows>().primary();
        let inner_size = uvec2(window.physical_width(), window.physical_height());
        let renderer = Renderer::new(inner_size).unwrap();
        Self(renderer)
    }
}

#[derive(Debug, Copy, Clone, Resource)]
pub struct ClearColor(pub Vec4);

impl Default for ClearColor {
    fn default() -> Self {
        Self(Vec3::ZERO.extend(1.))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Resource)]
pub struct RelativeViewport(pub Vec2, pub Vec2);

impl Default for RelativeViewport {
    fn default() -> Self {
        Self(Vec2::ZERO, Vec2::ONE)
    }
}

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.world.init_non_send_resource::<RenderTargetSurface>();
        app.world.init_non_send_resource::<OpenGlContext>();
        app.world.init_non_send_resource::<OpenGlDisplay>();

        let display = app.world.non_send_resource::<OpenGlDisplay>();
        violette::load_with(|sym| {
            let sym = CString::new(sym).unwrap();
            display.get_proc_address(sym.as_c_str())
        });
        violette::debug::hook_gl_to_tracing();

        app.world.init_non_send_resource::<RoseRenderer>();
        app.init_resource::<RelativeViewport>();
        app.init_resource::<ClearColor>();

        app.add_stage_before(
            CoreStage::Last,
            RenderStage::Init,
            RenderStage::schedule(),
        );
        app.add_startup_stage_before(StartupStage::Startup, RendererThread, SystemStage::single_threaded());
        app.add_stage_before(RenderStage::Init, RendererThread, SystemStage::single_threaded());
        app.add_system_to_stage(CoreStage::Last, swap_buffers);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, StageLabel)]
pub struct RendererThread;

#[derive(Debug, Copy, Clone, Eq, PartialEq, SystemLabel)]
pub enum RenderLabels {
    RenderToScreen,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, StageLabel)]
pub enum RenderStage {
    Init,
    PostInit,
    Submit,
    PreFlush,
    Flush,
}

impl RenderStage {
    fn schedule() -> Schedule {
        use RenderStage::*;
        Schedule::default()
            .with_stage(Init, SystemStage::single(begin_render))
            .with_stage(PostInit, SystemStage::parallel())
            .with_stage_after(
                PostInit,
                Submit,
                SystemStage::parallel().with_system(discover_meshes),
            )
            .with_stage_after(Submit, PreFlush, SystemStage::parallel().with_system(update_camera))
            .with_stage_after(
                PreFlush,
                Flush,
                SystemStage::single_threaded()
                    .with_system(renderer_resize)
                    .with_system(update_lights_buffer)
                    .with_system(render_to_screen.label(RenderLabels::RenderToScreen)),
            )
    }
}

fn begin_render(mut renderer: ResMut<RoseRenderer>) {
    renderer.begin_render().unwrap();
}

fn discover_meshes(
    mut renderer: ResMut<RoseRenderer>,
    meshes: Query<(&GlobalTransform, &Mesh, &Material)>,
) {
    for (transform, mesh, material) in &meshes {
        renderer.submit_mesh(
            material.weak_ref(),
            mesh.weak_ref()
                .transformed(rose_core::transform::Transform::from_matrix(
                    transform.compute_matrix(),
                )),
        )
    }
}

fn update_lights_buffer(
    mut lights_hash: Local<Option<u64>>,
    mut renderer: ResMut<RoseRenderer>,
    lights: Query<(&GlobalTransform, &Light)>,
) {
    let new_hash = {
        let mut hasher = DefaultHasher::new();
        for (transform, light) in &lights {
            transform.reflect_hash().hash(&mut hasher);
            light.hash(&mut hasher);
        }
        hasher.finish()
    };
    let redo_light_buffer = lights_hash
        .as_ref()
        .copied()
        .map(|prev_hash| prev_hash != new_hash)
        .unwrap_or(true);
    if redo_light_buffer {
        let lightbuffer = GpuLight::create_buffer(lights.iter().map(|(transform, light)| {
            match light.kind {
                LightKind::Ambient => rose_core::light::Light::Ambient {
                    color: light.color * light.power,
                },
                LightKind::Directional => rose_core::light::Light::Directional {
                    color: light.color * light.power,
                    dir: transform
                        .to_scale_rotation_translation()
                        .1
                        .mul_vec3(-Vec3::Z),
                },
                LightKind::Point => rose_core::light::Light::Point {
                    color: light.color * light.power,
                    position: transform.to_scale_rotation_translation().2,
                },
            }
        }));
        let lightbuffer = match lightbuffer {
            Ok(buf) => buf,
            Err(err) => {
                tracing::error!("Could not update lights: {}", err);
                return;
            }
        };
        renderer.set_light_buffer(lightbuffer);
        lights_hash.replace(new_hash);
    }
}

fn update_camera(mut renderer: NonSendMut<RoseRenderer>, active_camera_query: Query<(&GlobalTransform, &Camera), With<ActiveCamera>>) {
    if let Ok((transform, camera)) = active_camera_query.get_single() {
        let renderer_camera = renderer.camera_mut();
        renderer_camera.transform = transform.compute_matrix();
        renderer_camera.projection.zrange = camera.zrange.clone();
        renderer_camera.projection.fovy = camera.fovy;
    }
}

fn renderer_resize(
    windows: NonSend<Windows>,
    mut renderer: ResMut<RoseRenderer>,
    mut resize_events: EventReader<WindowResized>,
) {
    if let Some(event) = resize_events.iter().last() {
        let window = windows.get(event.id).unwrap();
        let size = vec2(event.width, event.height);
        let sizeu = (size * window.scale_factor() as f32).as_uvec2();
        renderer.resize(sizeu).unwrap();

        let proj = &mut renderer.camera_mut().projection;
        proj.width = event.width;
        proj.height = event.height;
    }
}

fn render_to_screen(
    mut renderer: ResMut<RoseRenderer>,
    clear_color: Res<ClearColor>,
    time: Res<Time>,
) {
    renderer.flush(clear_color.0, time.delta()).unwrap();
}

fn swap_buffers(surface: NonSend<RenderTargetSurface>, context: NonSend<OpenGlContext>) {
    surface.swap_buffers(&context).unwrap();
}

#[derive(Debug, Clone, Component)]
pub struct Mesh(Arc<rose_renderer::Mesh>);

impl From<rose_renderer::Mesh> for Mesh {
    fn from(mesh: rose_renderer::Mesh) -> Self {
        Self(Arc::new(mesh))
    }
}

impl Mesh {
    fn weak_ref(&self) -> Weak<rose_renderer::Mesh> {
        Arc::downgrade(&self.0)
    }
}

#[derive(Debug, Clone, Component)]
pub struct Material(Arc<rose_core::material::Material>);

impl From<rose_core::material::Material> for Material {
    fn from(mat: rose_core::material::Material) -> Self {
        Self(Arc::new(mat))
    }
}

impl Material {
    fn weak_ref(&self) -> Weak<rose_core::material::Material> {
        Arc::downgrade(&self.0)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum LightKind {
    Ambient,
    Directional,
    Point,
}

#[derive(Debug, Clone, Component)]
pub struct Light {
    pub kind: LightKind,
    pub color: Vec3,
    pub power: f32,
}

impl Hash for Light {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        bytemuck::bytes_of(&self.color).hash(state);
        self.power.to_le_bytes().hash(state);
    }
}

#[derive(Bundle)]
pub struct MeshBundle {
    pub global_transform: GlobalTransform,
    pub transform: Transform,
    pub mesh: Mesh,
    pub material: Material,
}

impl Default for MeshBundle {
    fn default() -> Self {
        Self {
            global_transform: GlobalTransform::default(),
            transform: Transform::default(),
            mesh: Mesh(DEFAULT_MESH.clone()),
            material: Material(DEFAULT_MATERIAL.clone()),
        }
    }
}

static DEFAULT_MESH: Lazy<Arc<rose_renderer::Mesh>> =
    Lazy::new(|| Arc::new(MeshBuilder::new(Vertex::new).uv_sphere(0.1, 8, 4).unwrap()));
static DEFAULT_MATERIAL: Lazy<Arc<rose_core::material::Material>> = Lazy::new(|| {
    Arc::new(rose_core::material::Material::create([1., 0., 1.], None, [0.2, 0.]).unwrap())
});

#[derive(Bundle)]
pub struct LightBundle {
    pub global_transform: GlobalTransform,
    pub transform: Transform,
    pub light: Light,
}

impl Default for LightBundle {
    fn default() -> Self {
        Self {
            global_transform: GlobalTransform::default(),
            transform: Transform::default(),
            light: Light {
                kind: LightKind::Ambient,
                color: vec3(1., 0., 1.),
                power: 0.1,
            },
        }
    }
}

#[derive(Component)]
pub struct Camera {
    pub fovy: f32,
    pub zrange: Range<f32>,
}

#[derive(Component)]
pub struct ActiveCamera;

impl Default for Camera {
    fn default() -> Self {
        Self {
            fovy: 45f32.to_radians(),
            zrange: 1e-3..1e3,
        }
    }
}

#[derive(Bundle, Default)]
pub struct CameraBundle {
    pub global_transform: GlobalTransform,
    pub transform: Transform,
    pub camera: Camera,
}