use std::ffi::CString;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    ops,
    sync::{Arc, Weak},
};

use bevy::{prelude::*, window::WindowResized};
use glam::{uvec2, vec2};
use glutin::{
    config::{Api, Config, ConfigTemplateBuilder},
    context::{
        ContextApi, ContextAttributesBuilder, GlProfile, NotCurrentContext, PossiblyCurrentContext,
        Robustness::RobustLoseContextOnReset, Version,
    },
    display::{Display, GetGlDisplay},
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use tracing::field::display;
use winit::event_loop::EventLoop;

use rose_core::{
    light::GpuLight,
    transform::{Transform, TransformExt},
};
use rose_renderer::Renderer;

pub struct OpenGlConfig(Config);

impl FromWorld for OpenGlConfig {
    fn from_world(world: &mut World) -> Self {
        let event_loop = world.non_send_resource::<EventLoop<()>>();
        let windows = world.resource::<Windows>();
        let window = windows.primary();
        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_depth_size(24)
            // .with_float_pixels(true)
            .with_api(Api::OPENGL);
        let (_, config) = DisplayBuilder::new()
            .build(event_loop, template, |mut configs| configs.next().unwrap())
            .unwrap();
        let ctx_attrs = ContextAttributesBuilder::new()
            .with_debug(cfg!(debug_assertions))
            .with_profile(GlProfile::Core)
            .with_robustness(RobustLoseContextOnReset)
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .build(Some(window.raw_handle().unwrap().window_handle));
        let not_current_context = unsafe {
            config
                .display()
                .create_context(&config, &ctx_attrs)
                .expect("Cannot create OpenGL context")
        };
        world.insert_non_send_resource(not_current_context);
        Self(config)
    }
}

pub struct RenderTargetSurface(Surface<WindowSurface>);

impl ops::Deref for RenderTargetSurface {
    type Target = Surface<WindowSurface>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromWorld for RenderTargetSurface {
    fn from_world(world: &mut World) -> Self {
        let windows = world.resource::<Windows>();
        let window = windows.primary();
        let config = world.non_send_resource::<OpenGlConfig>();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window.raw_handle().unwrap().window_handle,
            window.physical_width().try_into().unwrap(),
            window.physical_height().try_into().unwrap(),
        );
        let surface =
            unsafe { config.0.display().create_window_surface(&config.0, &attrs) }.unwrap();
        Self(surface)
    }
}

pub struct OpenGlContext(PossiblyCurrentContext);

impl ops::Deref for OpenGlContext {
    type Target = PossiblyCurrentContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromWorld for OpenGlContext {
    fn from_world(world: &mut World) -> Self {
        let context = world
            .remove_non_send_resource::<NotCurrentContext>()
            .unwrap();
        let surface = &*world.non_send_resource::<RenderTargetSurface>();
        Self(context.make_current(surface).unwrap())
    }
}

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

pub struct RendererPlugin;

impl Plugin for RendererPlugin {
    fn build(&self, app: &mut App) {
        app.world.init_non_send_resource::<OpenGlConfig>();
        app.world.init_non_send_resource::<RenderTargetSurface>();
        app.world.init_non_send_resource::<OpenGlContext>();

        let display = app.world.non_send_resource::<OpenGlConfig>().0.display();
        violette::debug::hook_gl_to_tracing();
        violette::load_with(|sym| {
            let sym = CString::new(sym).unwrap();
            display.get_proc_address(sym.as_c_str())
        });

        app.world.init_non_send_resource::<RoseRenderer>();

        app.init_resource::<ClearColor>();

        app.add_stage_after(
            CoreStage::Update,
            RenderStage::Init,
            RenderStage::schedule(),
        );
    }
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
            .with_system_in_stage(Init, begin_render)
            .with_stage(PostInit, SystemStage::parallel())
            .with_stage_after(
                PostInit,
                Submit,
                SystemStage::parallel().with_system(discover_meshes),
            )
            .with_stage_after(
                Submit,
                PreFlush,
                SystemStage::parallel()
                    .with_system(update_lights_buffer)
                    .with_system(renderer_resize),
            )
            .with_stage_after(PreFlush, Flush, SystemStage::single(end_render))
    }
}

fn begin_render(mut renderer: ResMut<RoseRenderer>, clear_color: Res<ClearColor>) {
    renderer.begin_render(clear_color.0).unwrap();
}

fn discover_meshes(
    mut renderer: ResMut<RoseRenderer>,
    meshes: Query<(&GlobalTransform, &Mesh, &Material)>,
) {
    for (transform, mesh, material) in &meshes {
        renderer.submit_mesh(
            material.weak_ref(),
            mesh.weak_ref()
                .transformed(Transform::from_matrix(transform.compute_matrix())),
        )
    }
}

fn update_lights_buffer(
    mut lights_hash: Local<Option<u64>>,
    mut renderer: ResMut<RoseRenderer>,
    lights: Query<(&GlobalTransform, &Light)>,
) {
    let mut hasher = DefaultHasher::new();
    for (transform, light) in &lights {
        transform.reflect_hash().hash(&mut hasher);
        light.hash(&mut hasher);
    }
    if let Some(prev_hash) = *lights_hash {
        let new_hash = hasher.finish();
        if prev_hash != new_hash {
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
}

fn renderer_resize(
    mut renderer: ResMut<RoseRenderer>,
    mut resize_events: EventReader<WindowResized>,
) {
    if let Some(event) = resize_events.iter().last() {
        let size = vec2(event.width, event.height);
        renderer.resize(size.as_uvec2()).unwrap();
    }
}

fn end_render(mut renderer: ResMut<RoseRenderer>, time: Res<Time>) {
    renderer.flush(time.delta()).unwrap();
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
