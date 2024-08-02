use std::{any::TypeId, cell::Cell, sync::Arc};
use ahash::AHashMap;
use parking_lot::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use thread_local::ThreadLocal;
use crate::{Read, Resource, ResourceMask, WorldBorrowError, WorldBorrowMutError, Write};

pub(crate) struct MutexedData {
    new: Vec<(TypeId, Box<dyn Resource>)>,
    remove: Vec<TypeId>,
    read: ResourceMask,
    write: ResourceMask,
}

impl MutexedData {
    pub(crate) fn new(read: ResourceMask, write: ResourceMask) -> Self {
        Self {
            new: Vec::default(),
            remove: Vec::default(),
            read,
            write,
        }
    }
}

#[derive(Default)]
pub struct World {
    pub(crate) resources: AHashMap<TypeId, RwLock<Box<dyn Resource>>>,
    pub(crate) local: ThreadLocal<Mutex<MutexedData>>,
}

impl World {
    // Insert a new resource into the world
    // This resource will be visible to the next execution group, not the current one
    pub fn insert<R: Resource>(&self, resource: R) {
        let id = TypeId::of::<R>();
        let mut local = self.local.get().unwrap().lock();
        local.new.push((id, Box::new(resource)));
    }

    // Get an immutable reference (read guard) to a resource
    pub fn get<R: Resource>(&self) -> Result<Read<R>, WorldBorrowError> {
        let mask = self.local.get().unwrap().lock().read;
        if (mask & R::mask()) == 0 {
            return Err(WorldBorrowError::InvalidAccess);
        }

        let cell = self
            .resources
            .get(&TypeId::of::<R>())
            .ok_or(WorldBorrowError::NotPresent)?;
        let mapped = RwLockReadGuard::map(cell.read(), |boxed| {
            boxed.as_any_ref().downcast_ref::<R>().unwrap()
        });
        Ok(Read(mapped))
    }

    // Get a mutable reference (write guard) to a resource
    pub fn get_mut<R: Resource>(&self) -> Result<Write<R>, WorldBorrowMutError> {
        let mask = self.local.get().unwrap().lock().write;
        if (mask & R::mask()) == 0 {
            return Err(WorldBorrowMutError::InvalidAccess);
        }

        let cell = self
            .resources
            .get(&TypeId::of::<R>())
            .ok_or(WorldBorrowMutError::NotPresent)?;
        let mapped = RwLockWriteGuard::map(cell.write(), |boxed| {
            boxed.as_any_mut().downcast_mut::<R>().unwrap()
        });
        Ok(Write(mapped))
    }

    // Remove a specific resource from the world
    pub fn remove<R: Resource>(&mut self) {
        self.local.get().unwrap().lock().remove.push(TypeId::of::<R>());
    }

    // Check if a resource is present in the world
    pub fn contains<R: Resource>(&self) -> bool {
        self.resources.contains_key(&TypeId::of::<R>())
    }
}