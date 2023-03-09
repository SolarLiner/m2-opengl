use std::ops::Range;

use crevice::std140::AsStd140;
use eyre::Result;
use glam::{Mat4, Vec2, Vec3, vec4, Vec4};

use violette::buffer::UniformBuffer;

use crate::transform::Transform;

#[derive(Debug, Clone)]
pub struct Projection {
    pub fovy: f32,
    pub width: f32,
    pub height: f32,
    pub zrange: Range<f32>,
}

impl Default for Projection {
    fn default() -> Self {
        Self {
            fovy: 45f32.to_radians(),
            zrange: 0.001..1000.0,
            width: 1.,
            height: 1.,
        }
    }
}

impl Projection {
    pub fn update(&mut self, size: Vec2) {
        self.width = size.x;
        self.height = size.y;
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::perspective_rh_gl(
            self.fovy,
            self.width / self.height,
            self.zrange.start,
            self.zrange.end,
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct Camera {
    pub transform: Transform,
    pub projection: Projection,
}

pub type ViewUniformBuffer = UniformBuffer<<ViewUniform as AsStd140>::Output>;

#[derive(Debug, Copy, Clone, AsStd140)]
pub struct ViewUniform {
    pub mat_view: Mat4,
    pub mat_proj: Mat4,
    pub inv_view: Mat4,
    pub inv_proj: Mat4,
    pub viewport: Vec4,
    pub camera_pos: Vec3,
}

impl ViewUniform {
    pub fn update_from_camera(&mut self, camera: &Camera) {
        self.mat_view = Mat4::from_rotation_translation(camera.transform.rotation, camera.transform.position);
        self.mat_proj = camera.projection.matrix();
        self.inv_view = self.mat_view.inverse();
        self.inv_proj = self.mat_proj.inverse();
        self.viewport = vec4(0., 0., camera.projection.width, camera.projection.height);
        self.camera_pos = camera.transform.position;
    }
}

impl ViewUniform {
    pub fn default_buffer() -> Result<ViewUniformBuffer> {
        Self::default().create_buffer()
    }

    pub fn create_buffer(&self) -> Result<ViewUniformBuffer> {
        ViewUniformBuffer::with_data(&[self.as_std140()])
    }
}

impl Default for ViewUniform {
    fn default() -> Self {
        Self::from(Camera::default())
    }
}

impl From<Camera> for ViewUniform {
    fn from(value: Camera) -> Self {
        let view = value.transform.matrix();
        let proj = value.projection.matrix();
        Self {
            mat_view: view,
            mat_proj: proj,
            inv_view: view.inverse(),
            inv_proj: proj.inverse(),
            viewport: vec4(0., 0., value.projection.width, value.projection.height),
            camera_pos: value.transform.position,
        }
    }
}

impl ViewUniform {
    pub fn new(camera: &Camera) -> Self {
        let mut this = Self::default();
        this.update_from_camera(camera);
        this
    }

    pub fn update_uniform_buffer(&self, buf: &mut ViewUniformBuffer) -> Result<()> {
        let mut slice = buf.at(0);
        slice.set(0, &self.as_std140())?;
        Ok(())
    }
}
