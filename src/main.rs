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
struct ResourceC;
struct Scene;
struct Renderer;
struct Graphics;
struct Audio;
struct Hierarchy;
struct Physics;
struct Input;
struct IO;

// must execute after B
fn system_a(w: &World) {}

// rando execution
fn system_b(w: &World) {}

// rando execution (could merge with B)
fn system_c(w: &World) {}

// rando execution (could merge with B)
fn system_d(w: &World) {}

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
    /*
    registry.insert(system_a)
        .after(system_c)
        .before(system_b);

    // insert system B, which does nothing... :3
    registry.insert(system_b);

    registry.insert(system_c)
        .reads(ResourceA::mask())
        .reads(ResourceB::mask());

    registry.insert(system_d)
        .writes(ResourceC::mask())
        .writes(ResourceA::mask());
    */

    registry.insert(|w| {})
        .writes(Scene::mask())
        .reads(Scene::mask() | Graphics::mask());
    registry.insert(|w| {})
    .writes(Scene::mask())
        .reads(Graphics::mask());
    registry.insert(|w| {})
        .writes(Scene::mask());
    registry.insert(|w| {})
        .reads(Scene::mask())
        .writes(Audio::mask());
    registry.insert(|w| {})
        .reads(Scene::mask())
        .reads(Input::mask())
        .writes(IO::mask());

    registry.insert(|w| {});
    
    let dispatcher = registry.sort();
    dispatcher.dispatch(6, Arc::new(World::default()))
    
    // setup system dag with injection systems
    
    // split systems across multiple virtual thread limit using their resources
    // during execution:
    //   split the executions across threads
}
