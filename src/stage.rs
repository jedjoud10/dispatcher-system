use std::{
    any::{type_name, TypeId},
    fmt::Debug,
};

use crate::world::World;

#[derive(Clone, Copy, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct StageId {
    pub name: &'static str,
    pub id: TypeId,
}

impl Debug for StageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("\"{}\"", &self.name))
    }
}

impl StageId {
    pub fn of<S: FnOnce(&World) + 'static>(_: &S) -> Self {
        Self {
            name: type_name::<S>(),
            id: TypeId::of::<S>(),
        }
    }
}
