use std::{any::TypeId, marker::PhantomData, sync::{Arc, Barrier}, thread::ThreadId};
use ahash::AHashMap;
use petgraph::{visit::Topo, Graph};

use crate::{dispatcher::Dispatcher, inject::InjectionOrder, rules::{default_rules, post_user, user, InjectionRule}, stage::StageId, world::World, ResourceMask};

pub(crate) struct Internal {
    pub(crate) boxed: Box<dyn FnMut(&World)>,
    pub(crate) rules: Vec<InjectionRule>,
    pub(crate) reads: ResourceMask,
    pub(crate) writes: ResourceMask,
}

#[derive(Default)]
pub struct UnfinishedRegistry<E> {
    _phantom: PhantomData<E>,
    systems: AHashMap<StageId, Internal>,
}

impl<E> UnfinishedRegistry<E> {
    pub fn insert<S: FnMut(&World) + 'static>(&mut self, system: S) -> InjectionOrder<E> {
        let rules = default_rules();
        let stage = StageId::fetch(&system);
        self.systems.insert(stage, Internal {
            boxed: Box::new(system),
            rules,
            reads: ResourceMask::default(),
            writes: ResourceMask::default(),
        });
        let internal = self.systems.get_mut(&stage).unwrap();
        InjectionOrder::new(internal)
    }

    // two (three) constraints
    // 1) make sure ordering constraint is held (sys a before sys b)
    // 2) make sure no intersecting RW masks
    // 3) (optional) optimize RW masks to improve concurrency
    pub fn sort(self) -> Dispatcher {
        let map = self.systems.into_iter().collect::<Vec<_>>();
        let mut output = AHashMap::<StageId, usize>::new();
        let mut graph = Graph::<StageId, &InjectionRule>::new();

        let mut nodes = map
            .iter()
            .map(|node| (node.0, graph.add_node(node.0)))
            .collect::<AHashMap<_, _>>();

        let sid = StageId::fetch(&user);
        let user = graph.add_node(sid);
        nodes.insert(sid, user);

        let sid = StageId::fetch(&post_user);
        let post_user = graph.add_node(sid);
        nodes.insert(sid, post_user);

        for (node, internal) in map.iter() {
            for rule in internal.rules.iter() {
                let this = nodes[node];
                let reference = rule.reference();
                let reference = *nodes
                    .get(&reference)
                    .unwrap();

                match rule {
                    // dir: a -> b.
                    // dir: this -> reference
                    InjectionRule::Before(_) => graph.add_edge(this, reference, rule),

                    // dir: a -> b.
                    // dir: reference -> this
                    InjectionRule::After(_) => graph.add_edge(reference, this, rule),
                    InjectionRule::Parallel(_) => todo!(),
                };
            }
        }

        // Topoligcally sort the graph (stage ordering)
        let mut topo = Topo::new(&graph);
        let mut counter = 0;
        while let Some(node) = topo.next(&graph) {
            let balls = nodes.iter().find(|x| *x.1 == node).unwrap();
            output.insert(*balls.0, counter);
            counter += 1;
        }

        todo!()
    }
}