use std::sync::Arc;
use dispatcher_system::*;
use parking_lot::RwLock;

// must execute after B
fn system_a(w: &World) {
    dbg!("a");
}

// rando execution
fn system_b(w: &World) {}

// rando execution (could merge with B)
fn system_c(w: &World) {}

struct ResourceA();
struct ResourceB();
struct ResourceC();

fn main() {
    let mut registry = UnfinishedRegistry::<()>::default();
    registry.insert(system_a)
        .after(system_c);

    registry.insert(system_b)
        .reads(ResourceA::mask())
        .reads(ResourceB::mask());

    registry.insert(system_c)
        .reads(ResourceC::mask());

    let world = Arc::new(RwLock::new(World::default()));
    let mut dispatcher = registry.sort(world.clone());
    dispatcher.dispatch();
}