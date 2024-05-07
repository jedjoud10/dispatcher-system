mod resources;
mod stage;
mod rules;
mod world;
mod inject;
mod dispatcher;
mod registry;
use std::sync::Arc;

use petgraph::{visit::{Dfs, EdgeRef, Topo}, Direction, Graph};
use registry::*;
use resources::*;
use world::*;

struct ResourceA;
struct ResourceB;

// must execute after B
fn system_a(w: &World) {}

// rando execution
fn system_b(w: &World) {}

// rando execution (should merge with B)
fn system_c(w: &World) {}

fn main() {
    /*
    let mut graph = Graph::<&'static str, u32>::new();
    let s = graph.add_node("Start"); // 0
    let s4 = graph.add_node("System D"); // 1
    let s3 = graph.add_node("System C"); // 1
    let s2 = graph.add_node("System B depends on A"); // 2
    let s1 = graph.add_node("System A by itself"); // 1

    // start
    graph.add_edge(s, s2, 0);
    graph.add_edge(s, s3, 100);
    graph.add_edge(s, s1, 0);
    graph.add_edge(s, s4, 0);
    
    // b depends on a
    graph.add_edge(s1, s2, 0);

    let mut topo = Topo::new(&graph);
    while let Some(node) = topo.next(&graph) {    
        println!("{}", graph[node]);
    }
    */    

    let mut registry = UnfinishedRegistry::<()>::default();

    // insert system A, which must execute after system B, and must read from the "ResourceA" resource
    registry.insert(system_a)
        .after(system_b)
        .reads(ResourceA::mask());

    // insert system B, which must read from ResourceA
    registry.insert(system_b)
        .reads(ResourceA::mask());

    // insert system C, which must read from ResourceA
    registry.insert(system_c)
        .reads(ResourceA::mask());
    
    let dispatcher = registry.sort();
    dispatcher.dispatch(6, Arc::new(World::default()))
    
    // setup system dag with injection systems
    
    // split systems across multiple virtual thread limit using their resources
    // during execution:
    //   split the executions across threads
}
