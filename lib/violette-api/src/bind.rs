pub trait Bind {
    type Id: Copy;

    fn id(&self) -> Self::Id;
    fn bind(&self);
    fn unbind(&self);
}
