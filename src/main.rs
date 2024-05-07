mod resources;
mod stage;
mod rules;
mod world;
mod inject;
mod dispatcher;
mod registry;
use std::sync::Arc;

use petgraph::{visit::Topo, Graph};
use registry::*;
use resources::*;
use world::*;

struct ResourceA;
struct ResourceB;
fn system_a(w: &World) {}
fn system_b(w: &World) {}

fn main() {
    let mut graph = Graph::<&'static str, u32>::new();
    let s = graph.add_node("Start");
    let s3 = graph.add_node("System C while B executes");
    let s2 = graph.add_node("System B depends on A");
    let s1 = graph.add_node("System A by itself");

    // start
    graph.add_edge(s, s2, 0);
    graph.add_edge(s, s3, 100);
    graph.add_edge(s, s1, 0);
    
    // b depends on a
    graph.add_edge(s1, s2, 0);

    let mut topo = Topo::new(&graph);
    while let Some(node) = topo.next(&graph) {
        println!("{}", graph[node]);
    }
    

    /*
    let mut registry = UnfinishedRegistry::<()>::default();

    // insert system A, which must execute after system B, and must read from the "ResourceA" resource
    registry.insert(system_a)
        .after(system_b)
        .reads(ResourceA::mask());

    // insert system B, which must write to ResourceA
    registry.insert(system_b)
        .writes(ResourceA::mask());
    
    let dispatcher = registry.sort();
    dispatcher.dispatch(6, Arc::new(World::default()))
    */
    // setup system dag with injection systems
    
    // split systems across multiple virtual thread limit using their resources
    // during execution:
    //   split the executions across threads
}
