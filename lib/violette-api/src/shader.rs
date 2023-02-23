use crate::{base::Resource, bind::Bind, context::GraphicsContext, value::AsValueType};

pub trait AsUniform<S: ?Sized + ShaderModule>: Into<S::Uniform> + AsValueType {}

impl<S: ShaderModule, T: Into<S::Uniform> + AsValueType> AsUniform<S> for T {}

pub trait ShaderModule: Resource + Bind {
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
