use std::any::Any;
use std::time::Duration;

use crevice::std140::AsStd140;
use serde::Deserialize;

use rose::{
    core::{camera::ViewUniformBuffer, transform::*, utils::reload_watcher::*},
    prelude::*,
    renderer::{DrawMaterial, Mesh},
};
use violette::shader::{FragmentShader, VertexShader};
use violette::{
    buffer::UniformBuffer,
    framebuffer::Framebuffer,
    program::{Program, UniformBlockIndex, UniformLocation},
    FrontFace,
};

#[derive(AsStd140, Deserialize)]
#[serde(default)]
struct AtmosphereUniforms {
    center: Vec3,
    atm_radius: f32,
    ground_radius: f32,
    ground_albedo: Vec3,
    sun_dir: Vec3,
    sun_color: Vec3,
}

impl Default for AtmosphereUniforms {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            atm_radius: 6460e3,
            ground_radius: 6360e3,
            ground_albedo: Vec3::splat(0.1),
            sun_dir: Vec3::X,
            sun_color: Vec3::ZERO,
        }
    }
}

#[derive(Debug)]
struct AtmosphereMaterial {
    program: ThreadGuard<Program>,
    uniform: ThreadGuard<UniformBuffer<Std140AtmosphereUniforms>>,
    proxy: ReloadFileProxy,
    u_block_view: UniformBlockIndex,
    u_block_atm: UniformBlockIndex,
    u_model: UniformLocation,
}

impl DrawMaterial for AtmosphereMaterial {
    fn draw<'a>(
        &self,
        frame: &Framebuffer,
        view: &ViewUniformBuffer,
        meshes: &mut dyn Iterator<Item = Transformed<&'a Mesh>>,
    ) -> Result<()> {
        violette::set_front_face(FrontFace::Clockwise);
        self.program
            .bind_block(&view.slice(0..=0), self.u_block_view, 0)?;
        self.program
            .bind_block(&self.uniform.slice(0..=0), self.u_block_atm, 1)?;
        for mesh in meshes {
            self.program
                .set_uniform(self.u_model, mesh.transform.matrix())?;
            mesh.draw(&self.program, frame, false)?;
        }
        violette::set_front_face(FrontFace::CounterClockwise);
        Ok(())
    }

    fn eq_key(&self) -> usize {
        self.uniform.id.get() as _
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl AtmosphereMaterial {
    fn new(reload_watcher: &ReloadWatcher) -> Result<Self> {
        let vert_path = reload_watcher.base_path().join("mesh/mesh.vert.glsl");
        let frag_path = reload_watcher.base_path().join("sky/atmosphere.frag.glsl");
        let vert_glsl = glsl_preprocessor::load_and_parse(&vert_path)?;
        let frag_glsl = glsl_preprocessor::load_and_parse(&frag_path)?;
        let vert_shader = VertexShader::new_multiple(vert_glsl.iter().map(|(_, s)| s.as_str()))
            .with_context(|| {
                format!(
                    "File map:{}",
                    vert_glsl
                        .iter()
                        .map(|(p, _)| p.as_path())
                        .enumerate()
                        .map(|(i, path)| format!("\n\t{}: {}", i, path.display()))
                        .reduce(|mut s, m| {
                            s.push_str(&m);
                            s
                        })
                        .unwrap_or_default()
                )
            })?;
        let frag_shader = FragmentShader::new_multiple(frag_glsl.iter().map(|(_, s)| s.as_str()))
            .with_context(|| {
            format!(
                "File map:{}",
                vert_glsl
                    .iter()
                    .map(|(p, _)| p.as_path())
                    .enumerate()
                    .map(|(i, path)| format!("\n\t{}: {}", i, path.display()))
                    .reduce(|mut s, m| {
                        s.push_str(&m);
                        s
                    })
                    .unwrap_or_default()
            )
        })?;
        let program = Program::new()
            .with_shader(vert_shader.id)
            .with_shader(frag_shader.id)
            .link()?;
        let uniform = UniformBuffer::with_data(&[AtmosphereUniforms::default().as_std140()])?;

        let u_block_view = program.uniform_block("View");
        let u_block_atm = program.uniform_block("Atmosphere");
        let u_model = program.uniform("model");

        let proxy = reload_watcher.proxy([vert_path.as_path(), frag_path.as_path()]);
        Ok(Self {
            program: ThreadGuard::new(program),
            uniform: ThreadGuard::new(uniform),
            proxy,
            u_block_atm,
            u_block_view,
            u_model,
        })
    }

    fn update_material(world: &mut World) -> Result<()> {
        let (sun_dir, sun_color) = world
            .query::<(&GlobalTransform, &components::Light)>()
            .iter()
            .find_map(|(_, (tr, light))| {
                if light.kind == LightKind::Directional {
                    Some((tr.0.rotation.mul_vec3(Vec3::X), light.color * light.power))
                } else {
                    None
                }
            })
            .unwrap_or((Vec3::X, Vec3::ZERO));

        for (_, (uniforms, transform)) in world
            .query::<(&mut AtmosphereUniforms, &GlobalTransform)>()
            .iter()
        {
            uniforms.center = transform.0.position;
            uniforms.sun_dir = sun_dir;
            uniforms.sun_color = sun_color;
        }

        for (_, (material, uniforms)) in world.query::<(&mut Self, &AtmosphereUniforms)>().iter() {
            material.uniform.slice(0..).set(0, &uniforms.as_std140())?;
        }
        Ok(())
    }
}

struct Rotate(Vec3);

impl Rotate {
    fn update(world: &mut World, dt: Duration) {
        for (_, (transform, rotate)) in world.query::<(&mut Transform, &Self)>().iter() {
            let [a, b, c] = (dt.as_secs_f32() * rotate.0).yxz().to_array();
            transform.rotation *= Quat::from_euler(EulerRot::YXZ, a, b, c);
        }
    }
}

struct EarthApp {
    core_systems: CoreSystems,
    pan_orbit_system: PanOrbitSystem,
    scene: Scene,
}

impl Application for EarthApp {
    fn new(size: PhysicalSize<f32>, scale_factor: f64) -> Result<Self> {
        println!(
            "LD_LIBRARY_PATH={}",
            std::env::var("LD_LIBRARY_PATH").unwrap()
        );
        let sizeu = Vec2::from_array(size.into()).as_uvec2();
        let mut core_systems = CoreSystems::new(sizeu)?;
        core_systems
            .render
            .register_custom_material::<AtmosphereMaterial>();
        let mut scene = Scene::new("assets")?;

        let cache = scene.asset_cache().as_any_cache();
        scene.with_world_mut(|world| {
            let entity = world.spawn((Transform::default(), Rotate(Vec3::Y / (6. * 60.))));
            world.spawn_children(
                entity,
                [
                    EntityBuilder::new().add_bundle(ObjectBundle {
                        // transform: Transform::default().scaled(Vec3::splat(6360e3)),
                        transform: Transform::default(),
                        material: cache.load::<assets::Material>("materials.earth")?,
                        mesh: core_systems.render.primitive_sphere(cache),
                        active: Active,
                    }),
                    EntityBuilder::new().add_bundle(ObjectBundle::<
                        CustomMaterial<AtmosphereMaterial>,
                    > {
                        // transform: Transform::default().scaled(Vec3::splat(6460e3)),
                        transform: Transform::default().scaled(Vec3::splat(1.05)),
                        material: cache.get_or_insert(
                            "materials.earth.atmosphere",
                            CustomMaterial::new(AtmosphereMaterial::new(
                                core_systems.render.renderer.reload_watcher(),
                            )?),
                        ),
                        mesh: core_systems.render.primitive_sphere(cache),
                        active: Active,
                    }),
                ],
            );
            world.spawn(LightBundle {
                transform: Transform::translation(Vec3::X).looking_at(Vec3::ZERO),
                light: components::Light {
                    kind: LightKind::Directional,
                    color: Vec3::ONE,
                    power: 100.,
                },
                ..Default::default()
            });
            world.spawn(PanOrbitCameraBundle {
                transform: Transform::translation(Vec3::splat(8.)).looking_at(Vec3::ZERO),
                pan_orbit: PanOrbitCamera {
                    radius: 3.,
                    ..Default::default()
                },
                params: CameraParams {
                    fovy: 60f32.to_radians(),
                    zrange: 1e-3..1e4,
                },
                ..Default::default()
            });
            Ok::<_, eyre::Report>(())
        })?;

        Ok(Self {
            core_systems,
            pan_orbit_system: PanOrbitSystem::new(size.to_logical(scale_factor)),
            scene,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>, scale_factor: f64) -> Result<()> {
        self.core_systems.resize(size)?;
        self.pan_orbit_system
            .set_window_size(size.to_logical(scale_factor));
        Ok(())
    }

    fn interact(&mut self, event: WindowEvent) -> Result<()> {
        let _ = self.core_systems.on_event(event);
        Ok(())
    }

    fn tick(&mut self, ctx: TickContext) -> Result<()> {
        self.scene.with_world_mut(|world| {
            Rotate::update(world, ctx.dt);
            AtmosphereMaterial::update_material(world)?;
            Ok::<_, eyre::Report>(())
        })?;
        Ok(())
    }

    fn render(&mut self, ctx: RenderContext) -> Result<()> {
        self.core_systems.begin_frame();
        self.scene.with_world_mut(|world| {
            self.pan_orbit_system
                .on_frame(&self.core_systems.input.input, world);
        });
        self.core_systems.end_frame(Some(&mut self.scene), ctx.dt)
    }
}

fn main() -> Result<()> {
    run::<EarthApp>("Earth")
}
