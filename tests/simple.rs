use std::sync::Arc;

use dispatcher_system::*;

fn system_a(world: &World) {
    assert!(world.dispatched());
    *world.get_mut::<i32>().unwrap() += 1;
}

#[test]
fn main() {
    env_logger::Builder::from_default_env()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let mut registry = Registry::default();

    registry.insert(system_a).unwrap().before(user);

    let builder = registry.sort().unwrap();
    assert_eq!(builder.group(0), Some(&vec![StageId::of(&system_a)]));
}

#[test]
fn outside() {
    env_logger::Builder::from_default_env()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let mut registry = Registry::default();

    registry.insert(system_a).unwrap().writes::<i32>();

    let mut world = World::default();
    world.insert(0i32);
    let world = Arc::new(world);
    let mut dispatcher = registry.sort().unwrap().build(world.clone(), None);
    assert!(!world.dispatched());
    dispatcher.dispatch();
    assert!(!world.dispatched());
    assert_eq!(*world.get::<i32>().unwrap(), 1);
}
