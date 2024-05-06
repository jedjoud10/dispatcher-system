use std::any::{Any, TypeId};

pub struct ResourceMask {
    types: Vec<TypeId>
}

pub trait Resource: Any + 'static + Sync + Send {
    fn mask() -> ResourceMask { todo!() }
    fn as_any_ref(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: Any + 'static> Resource for T {
    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub type BoxedResource = Box<dyn Resource>;