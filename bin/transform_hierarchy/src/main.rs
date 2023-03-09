use eyre::Result;
use glam::{EulerRot, Quat, UVec2, vec3, Vec3};

use rose_core::transform::Transform;
use rose_ecs::{assets::ObjectBundle, prelude::*};
use rose_ecs::systems::hierarchy::{GlobalTransform, HierarchicalSystem};
use rose_platform::{Application, events::WindowEvent, PhysicalSize, RenderContext};
use rose_platform::events::VirtualKeyCode;

struct App {
    core_systems: CoreSystems,
    scene: Scene,
    local_camera: Entity,
    global_camera: Entity,
}

struct Rotate(Vec3);

struct GlobalCamera;

struct LocalCamera;

impl Application for App {
    #[tracing::instrument]
    fn new(size: PhysicalSize<f32>, scale_factor: f64) -> Result<Self> {
        let sizeu = UVec2::from_array(size.cast::<u32>().into());
        let mut core_systems = CoreSystems::new(sizeu)?;
        core_systems.persistence.register_component::<GlobalTransform>();
        // core_systems.render.renderer.set_environment(EnvironmentMap::load("assets/textures/table_mountain_2_puresky_4k.exr")?);
        let mut scene = Scene::new("assets")?;
        let cache = scene.asset_cache();
        let (global_camera, local_camera) = scene.with_world_mut(|world| {
            world.spawn(LightBundle {
                transform: Transform::translation(Vec3::ONE).looking_at(Vec3::ZERO),
                light: Light {
                    kind: LightKind::Directional,
                    color: Vec3::ONE,
                    power: 10.,
                },
                ..Default::default()
            });
            // Create a grid of cubes to better see translation
            for i in -5..5 {
                for j in -5..5 {
                    world.spawn(ObjectBundle {
                        transform: Transform::translation(vec3(i as _, 0., j as _))
                            .scaled(Vec3::splat(0.1)),
                        mesh: core_systems.render.primitive_cube(cache),
                        material: core_systems.render.default_material_handle(cache),
                        active: Active,
                    });
                }
            }
            let root = world.spawn(
                EntityBuilder::new()
                    .add_bundle(ObjectBundle {
                        transform: Transform::rotation(Quat::from_rotation_x(20f32.to_radians())),
                        mesh: core_systems.render.primitive_sphere(cache),
                        material: cache.load("materials.square_floor")?,
                        active: Active,
                    })
                    .add(Rotate(vec3(0., 0.1, 0.)))
                    .build(),
            );
            let entities = world.spawn_children(
                root,
                [
                    EntityBuilder::new().add_bundle(ObjectBundle::from_asset_cache(
                        cache,
                        Transform::translation(vec3(3., 0., 0.))
                            .rotated_deg(Vec3::Y * 180.)
                            .scaled(Vec3::splat(0.7)),
                        "objects.suzanne",
                    )?),
                    EntityBuilder::new().add_bundle(ObjectBundle::from_asset_cache(
                        cache,
                        Transform::translation(Vec3::Y * 2.)
                            .rotated_deg(Vec3::Y * 180.)
                            .scaled(Vec3::splat(0.7)),
                        "objects.suzanne",
                    )?),
                ],
            );
            let grandchild = world.spawn_child(
                entities[1],
                EntityBuilder::new().add_bundle(ObjectBundle::from_asset_cache(
                    cache,
                    Transform::translation(Vec3::NEG_Y)
                        .looking_at_and_up(Vec3::ZERO, Vec3::X)
                        .rotated_deg(Vec3::Y * 180.)
                        .scaled(Vec3::splat(0.5)),
                    "objects.suzanne",
                )?),
            );
            let global = world.spawn(
                EntityBuilder::new()
                    .add_bundle(CameraBundle {
                        transform: Transform::translation(vec3(1., 2., 3.)).looking_at(Vec3::ZERO),
                        ..Default::default()
                    })
                    .add(GlobalCamera)
                    .add(Rotate(Vec3::Y * 0.1))
                    .build(),
            );
            let local = world.spawn_child(
                grandchild,
                EntityBuilder::new()
                    .add_bundle(CameraBundle {
                        transform: Transform::translation(vec3(0., 1., -1.) * 1.5)
                            .looking_at(Vec3::ZERO),
                        ..Default::default()
                    })
                    .add(LocalCamera),
            );
            world.remove_one::<Active>(local).unwrap();
            let mut cmd = CommandBuffer::new();
            HierarchicalSystem.update::<Transform>(world, &mut cmd);
            cmd.run_on(world);
            Ok::<_, eyre::Report>((global, local))
        })?;
        scene.set_path("assets/__saved_transform_hierarchy.scene");
        core_systems.save_scene(&scene)?;
        Ok(Self {
            core_systems,
            scene,
            local_camera,
            global_camera,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>, scale_factor: f64) -> Result<()> {
        self.core_systems.resize(size)?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn interact(&mut self, event: WindowEvent) -> Result<()> {
        if !self.core_systems.on_event(event) {
            let keyboard = &self.core_systems.input.input.keyboard;
            if keyboard.state.just_pressed(&VirtualKeyCode::G) {
                self.scene.with_world(|_, cmd| {
                    cmd.remove_one::<Active>(self.local_camera);
                    cmd.insert_one(self.global_camera, Active);
                });
            } else if keyboard.state.just_pressed(&VirtualKeyCode::L) {
                self.scene.with_world(|_, cmd| {
                    cmd.remove_one::<Active>(self.global_camera);
                    cmd.insert_one(self.local_camera, Active);
                })
            }
        }
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn render(&mut self, ctx: RenderContext) -> Result<()> {
        self.core_systems.begin_frame();
        self.scene.with_world_mut(|world| {
            let mut q = world.query::<(&mut Transform, &Rotate)>();
            for (_, (transform, Rotate(rotate))) in q.iter() {
                let rotate = *rotate * ctx.dt.as_secs_f32();
                transform.rotation *= Quat::from_euler(EulerRot::YXZ, rotate.y, rotate.x, rotate.z);
            }
        });
        self.core_systems.end_frame(Some(&mut self.scene), ctx.dt)
    }
}

fn main() -> Result<()> {
    rose_platform::run::<App>("Transform hierarchy")
}
