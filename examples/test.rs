use dispatcher_system::*;
use std::sync::Arc;

fn system_a(world: &World) {
    let value = world.get::<u32>().unwrap();
    dbg!(*value);
}

fn system_b(world: &World) {
    let value = world.get::<u32>().unwrap();
    dbg!(*value);
}

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let mut registry = Registry::default();
    registry.insert(system_b).unwrap().reads::<u32>();
    registry.insert(system_a).unwrap().writes::<u32>();

    let mut world = World::default();
    world.insert(123u32);
    let mut dispatcher = registry.sort().unwrap().build(Arc::new(world), None);
    dispatcher.dispatch();
}
