use dispatcher_system::*;
use std::sync::Arc;

fn system_a(_: &World) {
}

fn main() {
    env_logger::Builder::from_default_env().filter_level(log::LevelFilter::Debug).init();

    // Create a registry and add the system (making sure to set the "reads" bitmask)
    let mut registry = UnfinishedRegistry::default();
    registry.insert(system_a).unwrap();

    // Create a test world and add the resource
    let world = World::default();
    
    // Create a dispatcher by sorting the registry and execute it
    let mut dispatcher = registry.sort().unwrap().build(Arc::new(world), None);
    dispatcher.dispatch();
}
