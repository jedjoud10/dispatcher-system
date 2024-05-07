use std::{any::TypeId, fmt::Display, hash::{Hash, Hasher}, marker::PhantomData, sync::{Arc, Barrier}, thread::ThreadId};
use ahash::AHashMap;
use ascii_table::AsciiTable;
use petgraph::{algo::k_shortest_path, graph::NodeIndex, visit::{Bfs, DfsPostOrder, Topo}, Direction, Graph};

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
        let mut output = AHashMap::<StageId, usize>::new();
        let mut graph = Graph::<StageId, &InjectionRule>::new();

        let mut temp_vec = self.systems.iter().collect::<Vec<_>>();
        temp_vec.sort_by_key(|x| x.0);
        let mut nodes = temp_vec.iter()
            .map(|node| (*node.0, graph.add_node(*node.0)))
            .collect::<AHashMap<_, _>>();

        let sid = StageId::fetch(&user);
        let user = graph.add_node(sid);
        nodes.insert(sid, user);

        let sid = StageId::fetch(&post_user);
        let post_user = graph.add_node(sid);
        nodes.insert(sid, post_user);

        for (node, internal) in temp_vec.iter() {
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
                };
            }
        }

        // Groups for each type of resource access
        // Must correspond to the "depth" of each nodes as we can't mix and match groups from different levels (otherwise it would fuck
        // with the first requirement of having proper depedency sorting)
        let mut groups = Vec::<(i32, ResourceMask, ResourceMask, Vec<NodeIndex>)>::default();

        // Get the "depths" of each node assuming constant edge weights
        // This allows us to get adjacent sibling systems that we can merge if we have correct resource masks
        let path = k_shortest_path(&graph, user, None, 1, |x| 1);
        let mut path_sorted = path.iter().map(|(idx, temp)| {
            (idx, temp, self.systems.get(&graph[*idx]).map(|a| a.reads.count_ones() + a.writes.count_ones()).unwrap_or_default())
        }).collect::<Vec<_>>();

        
        // TODO: Sort the graph to minimize system fragmentation so that we can merge as many systems as possible together
        // Must apply some "weight" to the nodes so that nodes WITHOUT dependants are executed first (if dependency tree allows)
        // This way we can execute all similar systems first, then we can start subdividing down

        // FIX: Easiest fix would be to sort the systems based on rule set (so systems with similar rules are together)
        // unfortunately this system shits itself when the user defines more rules than necessary (as that would change their hash and ordering n shit) 
        path_sorted.sort_by_key(|(x, _, count)| {
            if **x == user || **x == post_user {
                return 0;
            } 

            let id = graph[**x];
            let internal = &self.systems[&id];
            let mut hash = ahash::AHasher::default();
            internal.rules.hash(&mut hash);
            hash.finish()
        });

        for (&index, &depth, resource_acces_counts) in path_sorted.iter() {
            let (stage_id, _) = nodes.iter().find(|x| *x.1 == index).unwrap();
            
            let Some(internal) = self.systems.get(stage_id) else {
                continue;
            };
            
            println!("{} {} {}", graph[index].name, depth, resource_acces_counts);
            let node_reads = internal.reads;
            let node_writes = internal.writes;

            // must find group with the following requirements:
            // 1) same depth as current node depth
            // 2) non intersecting read/writes

            // within a group, there should be shared access to all read resources, but unique access to all write resources
            let group_index = groups.iter().position(|(group_depth, group_reads, group_writes, _)| {
                let deptho = *group_depth == depth;

                // check for ref-mut collisions
                let ref_mut_collisions = (node_reads | node_writes) & group_writes == 0 && ((node_writes) & group_reads == 0);

                // check for mut-mut collisions
                let mut_mut_collisions = (node_writes & group_writes) == 0;
            
                deptho && ref_mut_collisions && mut_mut_collisions
            });

            // if missing, add
            if let Some(group_index) = group_index {
                let (_, read, writes, nodes) = &mut groups[group_index];
                *read |= node_reads;
                *writes |= node_writes;
                nodes.push(index);
            } else {
                groups.push((depth, node_reads, node_writes, vec![index]));
            }
        }

        for (i, (depth, reads, writes, _)) in groups.iter().enumerate() {
            println!("Index: {i}, Depth {depth}, R: {:#06b}, W: {:#06b}", *reads, *writes)
        }

        // Groups that the threads should execute
        // Separated into the systems that should be executed in parallel


        // Topoligcally sort the graph (stage ordering)
        let mut topo = Topo::new(&graph);
        let mut counter = 0;
        let mut ascii_table = AsciiTable::default();
        let mut data = Vec::<Vec<String>>::default();
        
        for i in 0..6 {
            let a = format!("Thread: {i}");
            data.push(vec![a]);

        }
        
        let mut last_group = None;
        let mut exec_counter = 0;

        // column based table to know what to execute in parallel
        let mut parralellinino = Vec::<Vec::<StageId>>::default();
        

        while let Some(node) = topo.next(&graph) {
            if (node == user || node == post_user) {
                continue;
            }

            let (stage, _) = nodes.iter().find(|x| *x.1 == node).unwrap();
            output.insert(*stage, counter);

            //let name = &stage.name.split("::").last().unwrap();
            counter += 1;
            
            let mut thread = 0;
            
            let group = groups.iter().position(|x| x.3.contains(&node));
            if let Some(i) = group {
                thread = groups[i].3.iter().position(|x| *x == node).unwrap();
                if last_group == Some(i) {
                    parralellinino[exec_counter-1].push(*stage);
                } else {
                    exec_counter += 1;
                    parralellinino.push(vec![*stage]);
                }

                last_group = Some(i);
            }

            println!("System: {}, group: {:?}, depth: {}, thread {thread}", graph[node].name, group, path[&node]);
        }

        let mut ascii_table = AsciiTable::default();
        for (i, execs) in parralellinino.into_iter().enumerate() {
            ascii_table.column(i+1).set_header(format!("{i}"));

            for j in 0..6 {
                if let Some(x) = execs.get(j) {
                    let name = &x.name.split("::").last().unwrap();
                    data[j].push(name.to_string());
                } else {
                    data[j].push("__".to_string());
                }
            }
        }
        ascii_table.print(data);

        Dispatcher {
            per_thread: todo!(),
        }
    }
}