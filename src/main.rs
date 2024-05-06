#![allow(warnings)]

use std::any::TypeId;

use registry::IncompleteRegistry;
use resources::Resource;
use world::World;

mod world;
mod registry;
mod stage;
mod inject;
mod resources;

struct ResourceA;
struct ResourceB;

fn test(world: &World) {}

fn main() {
    let mut reg = IncompleteRegistry::<()>::default();

    reg.insert(test)
        .reads(ResourceA::mask())
        .writes(ResourceB::mask());
    
}
