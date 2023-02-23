use std::hash::{Hash, Hasher};
use std::ops;

pub use nalgebra_glm::{
    Qua as Quat, TMat2 as Mat2, TMat2x3 as Mat2x3, TMat2x4 as Mat2x4, TMat3 as Mat3, TMat3x2 as Mat3x2,
    TMat3x4 as Mat3x4, TMat4 as Mat4, TMat4x2 as Mat4x2, TMat4x3 as Mat4x3, TVec2 as Vec2,
    TVec3 as Vec3, TVec4 as Vec4,
    Scalar,
};
pub use nalgebra_glm as glm;
use num_traits::{Num, NumCast, ToPrimitive};

#[derive(Debug, Copy, Clone)]
pub struct Rect<T> {
    pos: Vec2<T>,
    size: Vec2<T>,
}

impl<T: Scalar + Hash> Hash for Rect<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pos.x.hash(state);
        self.pos.y.hash(state);
        self.size.x.hash(state);
        self.size.y.hash(state);
    }
}

impl<T> From<[T; 4]> for Rect<T> {
    fn from([x, y, w, h]: [T; 4]) -> Self {
        Self {
            pos: Vec2::new(x, y),
            size: Vec2::new(w, h),
        }
    }
}

impl<T: Scalar> Into<[T; 4]> for Rect<T> {
    fn into(self) -> [T; 4] {
        let [x, y]: [T; 2] = self.pos.into();
        let [w, h]: [T; 2] = self.size.into();
        [x, y, w, h]
    }
}

impl<T: Scalar + ToPrimitive> Rect<T> {
    pub fn cast<U: Scalar + NumCast>(self) -> Rect<U> {
        Rect {
            pos: self.pos.map(|x| U::from(x).unwrap()),
            size: self.size.map(|x| U::from(x).unwrap()),
        }
    }
}

impl<T: Scalar> Rect<T> {
    pub fn from_pos_size(bottom_left: Vec2<T>, size: Vec2<T>) -> Self {
        Self {
            pos: bottom_left,
            size,
        }
    }

    pub fn into_array(self) -> [T; 4] {
        self.into()
    }
}

impl<T: Scalar> Rect<T>
where
    Self: Copy,
{
    pub fn as_array(&self) -> [T; 4] {
        Into::into(*self)
    }
}

impl<T: Copy + Scalar + Num> Rect<T>
where
    Vec2<T>: ops::Add<Vec2<T>, Output = Vec2<T>>,
{
    pub fn left(&self) -> T {
        self.pos.x
    }
    pub fn right(&self) -> T {
        self.pos.x + self.size.x
    }
    pub fn bottom(&self) -> T {
        self.pos.y
    }
    pub fn top(&self) -> T {
        self.pos.y + self.size.y
    }

    pub fn bottom_left(&self) -> Vec2<T> {
        self.pos
    }

    pub fn bottom_right(&self) -> Vec2<T> {
        Vec2::new(self.right(), self.bottom())
    }

    pub fn top_left(&self) -> Vec2<T> {
        Vec2::new(self.left(), self.top())
    }

    pub fn top_right(&self) -> Vec2<T> {
        self.pos + self.size
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Color(pub [f32; 4]);

impl Color {
    pub const BLACK: Self = Self([0., 0., 0., 1.]);
}

impl From<[u8; 3]> for Color {
    fn from(arr: [u8; 3]) -> Self {
        let farr = arr.map(|x| x as f32 / 255.);
        Self::from(farr)
    }
}

impl From<[u8; 4]> for Color {
    fn from(arr: [u8; 4]) -> Self {
        let farr = arr.map(|x| x as f32 / 255.);
        Self::from(farr)
    }
}

impl From<[f32; 3]> for Color {
    fn from([r, g, b]: [f32; 3]) -> Self {
        Self([r, g, b, 1.])
    }
}

impl From<[f32; 4]> for Color {
    fn from(arr: [f32; 4]) -> Self {
        Self(arr)
    }
}

impl Into<[f32; 4]> for Color {
    fn into(self) -> [f32; 4] {
        self.0
    }
}

impl Color {
    pub fn as_array(&self) -> [f32; 4] {
        self.0
    }

    pub fn into_array(self) -> [f32; 4] {
        self.0
    }
}
