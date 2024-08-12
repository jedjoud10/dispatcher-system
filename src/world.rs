use crate::{Read, Resource, ResourceMask, WorldBorrowError, WorldBorrowMutError, Write};
use ahash::AHashMap;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{any::TypeId, cell::RefCell};

pub(crate) struct InternalData {
    pub read: ResourceMask,
    pub write: ResourceMask,
}

#[derive(Default)]
pub struct World {
    pub(crate) resources: AHashMap<TypeId, RwLock<Box<dyn Resource>>>,
}

impl World {
    thread_local! {
        static INTERNAL: RefCell<Option<InternalData>> = const { RefCell::new(None) };
    }

    pub fn insert<R: Resource>(&mut self, resource: R) {
        let id = TypeId::of::<R>();
        self.resources.insert(id, RwLock::new(Box::new(resource)));
    }

    pub(crate) fn set_internal(&self, data: InternalData) {
        World::INTERNAL.with_borrow_mut(|x| *x = Some(data));
    }

    // Youssef was here writing a dumb comment about how this code is so unordered and not friendly to the eyes <3
    // Get an immutable reference (read guard) to a resource
    pub fn get<R: Resource>(&self) -> Result<Read<R>, WorldBorrowError> {
        let mask = World::INTERNAL.with_borrow(|x| x.as_ref().unwrap().read);

        if (mask & R::mask()) == 0 {
            return Err(WorldBorrowError::InvalidAccess);
        }

        let cell = self
            .resources
            .get(&TypeId::of::<R>())
            .ok_or(WorldBorrowError::NotPresent)?;
        let mapped = RwLockReadGuard::map(cell.read(), |boxed| {
            (**boxed).as_any_ref().downcast_ref::<R>().unwrap()
        });
        Ok(Read(mapped))
    }

    // Get a mutable reference (write guard) to a resource
    pub fn get_mut<R: Resource>(&self) -> Result<Write<R>, WorldBorrowMutError> {
        let mask = World::INTERNAL.with_borrow(|x| x.as_ref().unwrap().write);
        if (mask & R::mask()) == 0 {
            return Err(WorldBorrowMutError::InvalidAccess);
        }

        let cell = self
            .resources
            .get(&TypeId::of::<R>())
            .ok_or(WorldBorrowMutError::NotPresent)?;
        let mapped = RwLockWriteGuard::map(cell.write(), |boxed| {
            (**boxed).as_any_mut().downcast_mut::<R>().unwrap()
        });
        Ok(Write(mapped))
    }

    // Check if a resource is present in the world
    pub fn contains<R: Resource>(&self) -> bool {
        self.resources.contains_key(&TypeId::of::<R>())
    }
}
