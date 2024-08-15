use dispatcher_system::*;
use std::sync::Arc;

// Simple test resource
struct ResourrceA(u32);

// This system will set the value of Option<ResourrceA>. basically acting as adding it to the world at runtime
fn system_a(w: &World) {
    w.get_mut::<Option<ResourrceA>>().unwrap().replace(ResourrceA(0));
}

// This system executes after system "a" and will read from the now initialized resource
fn system_b(w: &World) {
    let res = w.get::<Option<ResourrceA>>().unwrap().map(|x| x.as_ref().unwrap());
    dbg!(res.0);
}

fn main() {
    env_logger::Builder::from_default_env().filter_level(log::LevelFilter::Debug).init();

    // Create a registry and add the two systems (making sure to set the proper accesses)
    let mut registry = Registry::default();
    registry.insert(system_a).unwrap().writes::<Option<ResourrceA>>();
    registry.insert(system_b).unwrap().after(system_a).reads::<Option<ResourrceA>>();

    // Create a test world and add the resource
    let mut world = World::default();
    world.insert(Option::<ResourrceA>::None);

    // Create a dispatcher by sorting the registry and execute it
    let mut dispatcher = registry.sort().unwrap().build(Arc::new(world), None);
    dispatcher.dispatch();
}