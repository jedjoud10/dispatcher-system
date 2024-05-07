use std::any::{type_name, TypeId};

use crate::world::World;

#[derive(Clone, Copy, Hash, PartialOrd, Ord, PartialEq, Eq, Debug)]
pub struct StageId {
    pub name: &'static str,
    pub id: TypeId,
}

impl StageId {
    pub fn fetch<S: FnOnce(&World) + 'static>(_: &S) -> Self {
        Self {
            name: type_name::<S>(),
            id: TypeId::of::<S>(),
        }
    }
}