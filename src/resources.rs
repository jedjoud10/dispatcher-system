use std::any::{Any, TypeId};

#[derive(Default)]
pub struct ResourceMask {
    types: Vec<TypeId>
}
impl ResourceMask {
    pub(crate) fn add(&mut self, mask: ResourceMask) {
        self.types.extend(mask.types);
    }
}

pub trait Resource: Any + 'static + Sync + Send {
    fn mask() -> ResourceMask where Self: Sized {
        ResourceMask {
            types: vec![TypeId::of::<Self>()],
        }
    } 
    fn as_any_ref(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
impl<T: Any + Sync + Send + 'static> Resource for T {
    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub type BoxedResource = Box<dyn Resource>;