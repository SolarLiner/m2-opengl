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
