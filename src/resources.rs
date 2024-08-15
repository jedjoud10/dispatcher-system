use ahash::AHashMap;
use parking_lot::Mutex;
use std::{
    any::{Any, TypeId},
    sync::LazyLock,
};

pub type ResourceMask = u64;

pub trait Resource: Any + 'static + Sync + Send {
    fn mask() -> ResourceMask
    where
        Self: Sized,
    {
        // Check if we need to register
        let id = TypeId::of::<Self>();
        if REGISTERED.lock().contains_key(&id) {
            // Read normally
            let locked = REGISTERED.lock();
            *locked.get(&id).unwrap()
        } else {
            // Register the component
            let mut locked = REGISTERED.lock();
            let mut bit = NEXT.lock();

            // Le bitshifting
            let copy = *bit;
            locked.insert(TypeId::of::<Self>(), copy);
            *bit = copy.checked_shl(1).unwrap();
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

static NEXT: LazyLock<Mutex<u64>> = LazyLock::new(|| Mutex::new(1));
static REGISTERED: LazyLock<Mutex<AHashMap<TypeId, u64>>> =
    LazyLock::new(|| Mutex::new(AHashMap::default()));
