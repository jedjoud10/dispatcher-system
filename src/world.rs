use std::any::TypeId;

use ahash::AHashMap;
use parking_lot::RwLock;

use crate::BoxedResource;

#[derive(Default)]
pub struct World {
    pub resources: AHashMap<TypeId, RwLock<BoxedResource>>, 
}