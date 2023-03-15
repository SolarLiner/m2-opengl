use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use crevice::std140::AsStd140;
use glam::Mat4;

use violette::buffer::{BufferUsageHint, UniformBuffer};

#[derive(Debug, Clone)]
pub struct Bone {
    parent: RefCell<Weak<Bone>>,
    pub children: RefCell<Vec<Rc<Bone>>>,
    local_transform: Cell<Mat4>,
}

impl Bone {
    pub fn new(transform: Mat4) -> Rc<Self> {
        Rc::new(Self {
            parent: RefCell::new(Weak::new()),
            children: RefCell::new(vec![]),
            local_transform: Cell::new(transform),
        })
    }

    fn as_std140(&self) -> Std140GpuBone {
        Std140GpuBone {
            transform: self.global_transform().as_std140(),
            _pad0: Default::default(),
        }
    }

    pub fn add_child(self: &Rc<Self>, child: Rc<Self>) {
        child.parent.replace(Rc::downgrade(self));
        self.children.borrow_mut().push(child);
    }

    pub fn traverse<'rc>(self: &'rc Rc<Self>) -> Box<dyn 'rc + Iterator<Item = Rc<Self>>> {
        Box::new(
            std::iter::once(self.clone()).chain(
                self.children
                    .borrow()
                    .iter()
                    .flat_map(|child| child.traverse())
                    .collect::<Vec<_>>(),
            ),
        )
    }

    pub fn update_transform(&self, update: impl FnOnce(Mat4) -> Mat4) {
        self.local_transform.set(update(self.local_transform.get()));
    }

    pub fn update_buffer(
        self: &Rc<Self>,
        buffer: &mut UniformBuffer<Std140GpuBone>,
    ) -> eyre::Result<()> {
        let gpu_data = self
            .traverse()
            .map(|bone| bone.as_std140())
            .collect::<Vec<_>>();
        tracing::debug!(message = "Updating bone data", len = gpu_data.len());
        buffer.set(&gpu_data, BufferUsageHint::Stream)?;
        Ok(())
    }
}

impl Bone {
    pub fn global_transform(&self) -> Mat4 {
        if let Some(parent) = self.parent.borrow().upgrade() {
            parent.global_transform() * self.local_transform.get()
        } else {
            self.local_transform.get()
        }
    }
}

#[derive(Debug, Copy, Clone, AsStd140)]
pub struct GpuBone {
    transform: Mat4,
}

impl From<Bone> for GpuBone {
    fn from(value: Bone) -> Self {
        Self {
            transform: value.global_transform(),
        }
    }
}
