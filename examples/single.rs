use std::{any::Any, sync::Arc};
use dispatcher_system::*;
use parking_lot::RwLock;

struct ResourrceA;

fn system_a(w: &World) {
    w.get::<ResourrceA>().unwrap();
}

fn main() {
    let mut registry = UnfinishedRegistry::<()>::default();
    registry.insert(system_a).reads(ResourrceA::mask());

    let mut world = World::default();
    world.insert(ResourrceA);

    let world = Arc::new(RwLock::new(world));
    let mut dispatcher = registry.sort(world.clone());
    dispatcher.dispatch();
}