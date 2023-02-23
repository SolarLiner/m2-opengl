use std::{
    fmt,
    sync::Arc
};

pub trait Resource: Send + Sync + fmt::Debug {
    fn set_name(&self, name: impl ToString);
    fn get_name(&self) -> Option<String>;

    fn named(self: Arc<Self>, name: impl ToString) -> Arc<Self> {
        self.set_name(name);
        self
    }
}