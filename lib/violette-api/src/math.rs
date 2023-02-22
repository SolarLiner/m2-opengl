use std::ops;

use cgmath::{
    num_traits::{Num, NumCast, ToPrimitive},
    Vector2,
};

#[derive(Debug, Copy, Clone, Hash)]
pub struct Rect<T> {
    pos: Vector2<T>,
    size: Vector2<T>,
}

impl<T> From<[T; 4]> for Rect<T> {
    fn from([x, y, w, h]: [T; 4]) -> Self {
        Self {
            pos: Vector2::new(x, y),
            size: Vector2::new(w, h),
        }
    }
}

impl<T> Into<[T; 4]> for Rect<T> {
    fn into(self) -> [T; 4] {
        let [x, y]: [T; 2] = self.pos.into();
        let [w, h]: [T; 2] = self.size.into();
        [x, y, w, h]
    }
}

impl<T: ToPrimitive> Rect<T> {
    pub fn cast<U: NumCast>(self) -> Rect<U> {
        Rect {
            pos: self.pos.map(|x| U::from(x).unwrap()),
            size: self.size.map(|x| U::from(x).unwrap()),
        }
    }
}

impl<T> Rect<T> {
    pub fn from_pos_size(bottom_left: Vector2<T>, size: Vector2<T>) -> Self {
        Self {
            pos: bottom_left,
            size,
        }
    }

    pub fn into_array(self) -> [T; 4] {
        self.into()
    }
}

impl<T> Rect<T>
where
    Self: Copy,
{
    pub fn as_array(&self) -> [T; 4] {
        Into::into(*self)
    }
}

impl<T: Copy + Num> Rect<T>
where
    Vector2<T>: ops::Add<Vector2<T>, Output = Vector2<T>>,
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

    pub fn bottom_left(&self) -> Vector2<T> {
        self.pos
    }

    pub fn bottom_right(&self) -> Vector2<T> {
        Vector2::new(self.right(), self.bottom())
    }

    pub fn top_left(&self) -> Vector2<T> {
        Vector2::new(self.left(), self.top())
    }

    pub fn top_right(&self) -> Vector2<T> {
        self.pos + self.size
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Color(pub [f32; 4]);

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
