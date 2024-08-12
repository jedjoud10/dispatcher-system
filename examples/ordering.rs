use dispatcher_system::*;
use std::sync::Arc;

// must execute after C
fn system_a(w: &World) {}

// could execute with c
fn system_b(w: &World) {}

// could execute with b
fn system_c(w: &World) {}

// must execute after all of them
fn system_d(w: &World) {}

fn main() {
    env_logger::Builder::from_default_env().filter_level(log::LevelFilter::Debug).init();

    // Create a registry and add the systems with their rules
    let mut registry = UnfinishedRegistry::<()>::default();
    registry.insert(system_a).unwrap().after(system_c);
    registry.insert(system_b).unwrap();
    registry.insert(system_c).unwrap();
    registry.insert(system_d).unwrap().after(post_user);

    // Create a test world and add the resource
    let world = World::default();

    // Create a dispatcher by sorting the registry and execute it
    let mut dispatcher = registry.sort(Arc::new(world)).unwrap();
    dispatcher.dispatch();
}
