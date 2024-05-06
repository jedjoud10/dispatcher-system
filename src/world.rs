use std::{any::TypeId, collections::HashMap, sync::{Arc}};
use parking_lot::{lock_api::RwLockReadGuard, MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use thread_local::ThreadLocal;
use crate::resources::{BoxedResource, Resource};

// Sharded world sent to each system
// best case: fetch from local variable
// worst case: lock contentation when accessing global resource
pub struct World {
    global: Arc<HashMap<TypeId, RwLock<BoxedResource>>>,
}


impl World {
    // only insert it locally to thread local data. gets sent to main thread after ALL systems in this group have executed
    pub fn insert<R: Resource>(&self, resource: R) {
        self.local.get_or_default().borrow_mut().insert(TypeId::of::<R>(), Box::new(resource));
    }

    // checks thread local data first and then checks world data
    pub fn get<R: Resource>(&self) -> MappedRwLockReadGuard<R> {
        let id = &TypeId::of::<R>();

        //let local = self.local.get().map(|local| local.get(id)).flatten();
        let global = self.global.get(id)?.read();
        RwLockReadGuard::map(global, |x| x.as_any_ref().downcast_ref::<R>().unwrap())
    }

    // marks "local" as removed, then actually removes it from the world at the end of the group execution
    // if it is accessing world global data, then it simply sets it to None to avoid modifying the global hash map
    pub fn remove<R: Resource>(&self) -> Option<R> {
        let id = &TypeId::of::<R>();
        let data = self.local.get_or_default().borrow_mut();

        if !data.contains_key(id) {
            let boxed = self.global.get(id).unwrap().write().take()?;
            return Some(*boxed.downcast::<R>().unwrap());
        }

        data
    }

    // move all the new thread local resources to the main thread
}