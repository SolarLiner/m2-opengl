use duplicate::duplicate_item;
use crate::math::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ScalarType {
    Bool,
    I8, I16, I32, I64,
    U8, U16, U32, U64,
    F32, F64
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ValueType {
    Scalar(ScalarType),
    Vector(u8, ScalarType),
    Matrix(u8, u8, ScalarType),
}

pub trait AsScalarType {
    fn scalar_type() -> ScalarType;
}

pub trait AsValueType {
    fn value_type() -> ValueType;
}

impl<T: AsScalarType> AsValueType for T {
    fn value_type() -> ValueType {
        ValueType::Scalar(T::scalar_type())
    }
}

#[duplicate_item(
ty      scalar_ty;
[u8]    [U8];
[u16]   [U16];
[u32]   [U32];
[i8]    [I8];
[i16]   [I16];
[i32]   [I32];
[f32]   [F32];
[f64]   [F64];
)]
impl AsScalarType for ty {
    fn scalar_type() -> ScalarType { ScalarType::scalar_ty }
}

#[duplicate_item(
vec     num_components;
[Vec2]  [2];
[Vec3]  [3];
[Vec4]  [4];
)]
#[duplicate_item(
ty;
[u8];
[u16];
[u32];
[i8];
[i16];
[i32];
[f32];
)]
impl AsValueType for vec<ty> {
    fn value_type() -> ValueType {
        ValueType::Vector(num_components, ty::scalar_type())
    }
}

#[duplicate_item(
mat         n       m;
[Mat2]      [2]     [2];
[Mat2x3]    [2]     [3];
[Mat2x4]    [2]     [4];
[Mat3x2]    [3]     [2];
[Mat3]      [3]     [3];
[Mat3x4]    [3]     [4];
[Mat4x2]    [4]     [2];
[Mat4x3]    [4]     [3];
[Mat4]      [4]     [4];
)]
#[duplicate_item(
ty;
[u8];
[u16];
[u32];
[i8];
[i16];
[i32];
[f32];
)]
impl AsValueType for mat<ty> {
    fn value_type() -> ValueType {
        ValueType::Matrix(n, m, ty::scalar_type())
    }
}
