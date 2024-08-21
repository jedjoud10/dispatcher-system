#![allow(unused_must_use)]
use dispatcher_system::*;
use std::sync::Arc;

fn system_a(world: &World) {
    *world.get_mut::<i32>().unwrap() += 1;
}

fn system_b(world: &World) {
    *world.get_mut::<i32>().unwrap() -= 1;
}

fn system_c(world: &World) {
    assert!(matches!(world.get_mut::<i32>(), Err(WorldBorrowMutError::InvalidAccess)));
}

fn system_d(world: &World) {
    assert!(matches!(world.get_mut::<i32>(), Err(WorldBorrowMutError::NotPresent)));
}

#[test]
fn int() {
    env_logger::Builder::from_default_env()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let mut registry = Registry::default();

    registry
        .insert(system_a)
        .unwrap()
        .before(system_b)
        .writes::<i32>();
    registry.insert(system_b).unwrap().writes::<i32>();

    let mut world = World::default();
    world.insert(0i32);
    let world = Arc::new(world);

    let mut dispatcher = registry.sort().unwrap().build(world.clone(), None);
    dispatcher.dispatch();
    assert_eq!(*world.get::<i32>().unwrap(), 0);
}

#[test]
fn err() {
    env_logger::Builder::from_default_env()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let mut registry = Registry::default();

    registry.insert(system_c).unwrap();
    registry.insert(system_d).unwrap().writes::<i32>();
    let world = Arc::new(World::default());
    let mut dispatcher = registry.sort().unwrap().build(world.clone(), None);
    dispatcher.dispatch();
}
