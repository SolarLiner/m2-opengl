use bevy::prelude::*;
use bevy::transform::TransformSystem;
use dolly::prelude::*;

pub struct DollyPlugin;

impl Plugin for DollyPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(
            CoreStage::PostUpdate,
            update_camera_transform.before(TransformSystem::TransformPropagate),
        );
    }
}

#[derive(Debug, Component)]
pub struct DollyCamera(pub CameraRig);

fn update_camera_transform(time: Res<Time>, mut query: Query<(&mut Transform, &mut DollyCamera)>) {
    for (mut transform, mut camera) in &mut query {
        let DollyCamera(camera) = &mut *camera;
        camera.update(time.delta_seconds());
        transform.rotation = camera.final_transform.rotation;
        transform.translation = camera.final_transform.position;
    }
}
