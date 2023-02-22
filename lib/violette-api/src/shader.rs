use crate::bind::Bind;
use crate::context::GraphicsContext;
use crate::value::ValueType;

pub trait AsUniform<S: ?Sized + ShaderModule>: Into<S::Uniform> {
    fn value_type() -> ValueType;
}

pub trait ShaderModule: Bind + Send + Sync {
    type Gc: GraphicsContext;
    type Err: Into<<Self::Gc as GraphicsContext>::Err>;
    type ShaderSource;
    type Uniform;
    type UniformLocation;

    fn add_shader_source(&self, source: Self::ShaderSource) -> Result<(), Self::Err>;
    fn link(&self) -> Result<(), Self::Err>;
    fn uniform_location(&self, name: &str) -> Option<Self::UniformLocation>;
    fn set_uniform(&self, location: Self::UniformLocation, uniform: impl AsUniform<Self>);
}