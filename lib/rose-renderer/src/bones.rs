use crevice::std140::AsStd140;
use glam::Mat4;

#[derive(Debug, Copy, Clone, AsStd140)]
pub struct Bone {
    local_transform: Mat4,
}