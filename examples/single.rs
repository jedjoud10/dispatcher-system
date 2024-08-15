use dispatcher_system::*;
use std::sync::Arc;

// Simple test resource
struct ResourrceA(u32);

// This will read from the resource and print its internal value
fn system_a(w: &World) {
    let res = w.get::<ResourrceA>().unwrap();
    dbg!(res.0);
}

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // Create a registry and add the system (making sure to set the "reads" bitmask)
    let mut registry = Registry::default();
    registry.insert(system_a).unwrap();

    // Create a test world and add the resource
    let mut world = World::default();
    world.insert(ResourrceA(0));

    // Create a dispatcher by sorting the registry and execute it
    let mut dispatcher = registry.sort().unwrap().build(Arc::new(world), None);
    dispatcher.dispatch();
}
