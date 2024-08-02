use std::any::{Any, TypeId};
use ahash::AHashMap;
use lazy_static::lazy_static;
use parking_lot::{Mutex, RwLock};

pub type ResourceMask = u64;

pub trait Resource: Any + 'static + Sync + Send {
    fn mask() -> ResourceMask where Self: Sized {
        // Check if we need to register
        let id = TypeId::of::<Self>();
        if REGISTERED.read().contains_key(&id) {
            // Read normally
            let locked = REGISTERED.read();
            *locked.get(&id).unwrap()
        } else {
            // Register the component
            let mut locked = REGISTERED.write();
            let mut bit = NEXT.lock();

            // Le bitshifting
            let copy = *bit;
            locked.insert(TypeId::of::<Self>(), copy);
            *bit = u64::from(copy).checked_shl(1).unwrap();
            copy
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

// Registered components
lazy_static! {
    static ref NEXT: Mutex<u64> = Mutex::new(1);
    static ref REGISTERED: RwLock<AHashMap<TypeId, u64>> = RwLock::new(AHashMap::new());
}