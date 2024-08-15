use ahash::AHashMap;
use ascii_table::AsciiTable;
use petgraph::{algo::k_shortest_path, graph::{NodeIndex, UnGraph}, visit::{EdgeRef, NodeRef, Topo}, Graph};
use std::{
    collections::HashMap, hash::{Hash, Hasher}, marker::PhantomData, sync::Arc
};

use crate::{
    dispatcher::Dispatcher, inject::InjectionOrder, rules::{default_rules, post_user, user, InjectionRule}, stage::StageId, world::World, RegistrySortingError, ResourceMask, SortedRegistry, StageError
};

pub(crate) struct Internal {
    pub(crate) boxed: Box<dyn FnMut(&World) + Sync + Send>,
    pub(crate) stage: StageId,
    pub(crate) rules: Vec<InjectionRule>,
    pub(crate) reads: ResourceMask,
    pub(crate) writes: ResourceMask,
}


#[derive(Default)]
pub struct UnfinishedRegistry {
    systems: AHashMap<StageId, Internal>,
    thread_count: Option<usize>,
}

impl UnfinishedRegistry {
    // Add a new system to the registry so we can execute it
    pub fn insert<S: FnMut(&World) + Sync + Send + 'static>(
        &mut self,
        system: S,
    ) -> Result<InjectionOrder, StageError> {
        let rules = default_rules();
        let stage = StageId::of(&system);
        
        if self.systems.contains_key(&stage) {
            return Err(StageError::Overlapping);
        }

        if [StageId::of(&user), StageId::of(&post_user)].contains(&stage) {
            return Err(StageError::InvalidName);
        }

        self.systems.insert(
            stage,
            Internal {
                stage,
                boxed: Box::new(system),
                rules,
                reads: ResourceMask::default(),
                writes: ResourceMask::default(),
            },
        );
        let internal = self.systems.get_mut(&stage).unwrap();
        Ok(InjectionOrder::new(internal))
    }

    // Set the number of threads that the registry can use during dispatch calls
    // Set to None to use the current number of logical threads - 1
    pub fn set_thread_count(&mut self, thread_count: Option<usize>) {
        self.thread_count = thread_count;
    }

    // two (three) constraints
    // 1) make sure ordering constraint is held (sys a before sys b)
    // 2) make sure no intersecting RW masks
    // 3) (optional) optimize RW masks to improve concurrency
    pub fn sort(self) -> Result<SortedRegistry, RegistrySortingError> {
        let thread_count = self.thread_count.unwrap_or_else(|| num_cpus::get() - 1).max(1);
        let mut graph = Graph::<StageId, ()>::new();

        let mut temp_vec = self.systems.iter().collect::<Vec<_>>();
        temp_vec.sort_by_key(|x| x.0);
        let mut nodes = temp_vec
            .iter()
            .map(|node| (*node.0, graph.add_node(*node.0)))
            .collect::<AHashMap<_, _>>();

        let sid = StageId::of(&user);
        let user = graph.add_node(sid);
        nodes.insert(sid, user);

        let sid = StageId::of(&post_user);
        let post_user = graph.add_node(sid);
        nodes.insert(sid, post_user);
        graph.add_edge(user, post_user, ());

        for (node, internal) in temp_vec.iter() {
            for rule in internal.rules.iter() {
                let this = nodes[node];
                let reference = rule.reference();
                let reference = *nodes.get(&reference).ok_or(RegistrySortingError::MissingStage(**node, reference))?;

                match rule {
                    // dir: a -> b.
                    // dir: this -> reference
                    InjectionRule::Before(_) => graph.add_edge(this, reference, ()),

                    // dir: a -> b.
                    // dir: reference -> this
                    InjectionRule::After(_) => graph.add_edge(reference, this, ()),
                };
            }
        }

        // Groups for each type of resource access
        // Must correspond to the "depth" of each nodes as we can't mix and match groups from different levels (otherwise it would fuck
        // with the first requirement of having proper depedency sorting)
        let mut groups = Vec::<(i32, ResourceMask, ResourceMask, Vec<NodeIndex>)>::default();

        // Get the "depths" of each node assuming constant edge weights
        // This allows us to get adjacent sibling systems that we can merge if we have correct resource masks
        /*
        let path: HashMap<NodeIndex, i32> = calculate_depths(&graph, user);

        for (node, index) in &path {
            log::debug!("System: {}, Index: {}", graph.node_weight(*node).unwrap().name, index);
        }
        */

        #[derive(Debug, Clone)]
        enum Testino {
            Concrete(i32),
            Ref(NodeIndex),
        }

        let mut path_sorted = Vec::<(NodeIndex, i32)>::default();
        let mut test = AHashMap::<NodeIndex, Testino>::default();
        let mut bruh = Topo::new(&graph);
        while let Some(a) = bruh.next(&graph) {
            if test.is_empty() {
                test.insert(a, Testino::Concrete(0));
            }

            log::debug!("{:?}, Depth: {:?}", graph.node_weight(a).unwrap(), test.get(&a));
            if test.get(&a).is_some() {
                for x in graph.edges_directed(a, petgraph::Direction::Outgoing) {
                    let rizz = test.get(&a).map(|x| match x {
                        Testino::Concrete(u) => Testino::Concrete(u + 1),
                        Testino::Ref(u) => Testino::Ref(*u),
                    }).unwrap_or_else(|| Testino::Ref(a));
    
                    test.insert(x.target(), rizz);
                }
            }
            

            path_sorted.push((a, 0));
        }

        // oh god it is horrible please make it stop
        for (key, val) in test.iter() {
            let (_, depth_to_write) = path_sorted.iter_mut().find(|x| x.0 == *key).unwrap();
            
            let mut next = val.clone();
            let mut count = 0;
            while let Testino::Ref(idx) = next {
                next = test[&idx].clone();
                count += 1;
            }
            
            if let Testino::Concrete(k) = next {
                *depth_to_write = k + count;
            } else {
                panic!();
            }
        }

        /*
        for (key, val) in test {
            path_sorted[key]
        }
        */

        /*
        let mut path_sorted = path
            .iter()
            .map(|(idx, temp)| {
                (
                    idx,
                    self.systems
                        .get(&graph[*idx])
                        .map(|a| a.reads.count_ones() + a.writes.count_ones())
                        .unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>();
        */

        // TODO: Sort the graph to minimize system fragmentation so that we can merge as many systems as possible together
        // Must apply some "weight" to the nodes so that nodes WITHOUT dependants are executed first (if dependency tree allows)
        // This way we can execute all similar systems first, then we can start subdividing down

        // FIX: Easiest fix would be to sort the systems based on rule set (so systems with similar rules are together)
        // unfortunately this system shits itself when the user defines more rules than necessary (as that would change their key)
        /*
        path_sorted.sort_by_key(|(x, _, _)| {
            if **x == user || **x == post_user {
                return 0;
            }

            let id = graph[**x];
            let internal = &self.systems[&id];
            let mut hash = ahash::AHasher::default();
            internal.rules.hash(&mut hash);
            hash.finish()
        });
        */

        // this will batch the system into their "group" batches where they can execute in parallel with other systems
        // without overlapping their resource bitmasks
        for &(index, depth) in path_sorted.iter() {
            let (stage_id, _) = nodes.iter().find(|x| *x.1 == index).unwrap();

            let Some(internal) = self.systems.get(stage_id) else {
                continue;
            };

            let node_reads = internal.reads;
            let node_writes = internal.writes;
            log::debug!(
                "System: {}, Depth: {} R: {:#06b}, W: {:#06b}",
                graph[index].name,
                depth,
                node_reads,
                node_writes
            );

            // must find group with the following requirements:
            // 1) same depth as current node depth
            // 2) non intersecting read/writes

            // one could even write a custom heuristic function here to split off systems within groups if needed

            // within a group, there should be shared access to all read resources, but unique access to all write resources
            let group_index =
                groups
                    .iter()
                    .position(|(group_depth, group_reads, group_writes, _)| {
                        // check group depth
                        let depth = *group_depth == depth;
                        
                        // check for ref-mut collisions
                        let ref_mut_collisions = (node_reads | node_writes) & group_writes == 0
                            && ((node_writes) & group_reads == 0);

                        // check for mut-mut collisions
                        let mut_mut_collisions = (node_writes & group_writes) == 0;

                        depth && ref_mut_collisions && mut_mut_collisions
                    });

            // if the group is missing, add it, otherwise just modify the current group
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
            log::debug!(
                "Index: {i}, Depth {depth}, R: {:#06b}, W: {:#06b}",
                *reads,
                *writes
            )
        }

        // Topoligcally sort the graph (stage ordering)
        // We do this AFTER we try to merge all similar systems since trying to run them in parallel isn't as important as getting the ordering right
        let mut data = Vec::<Vec<String>>::default();
        for i in 0..thread_count {
            let a = format!("Thread: {}", i + 1);
            data.push(vec![a]);
        }

        // column based table to know what to execute in parallel
        let mut execution_matrix_cm = Vec::<Vec<StageId>>::default();
        let mut count = 0;

        groups.sort_by_key(|(depth, _, _, _)| *depth);

        for (_, _, _, x) in groups.iter() {
            let g = x.iter().map(|a| *graph.node_weight(*a).unwrap()).collect::<Vec<_>>();
            count += g.len();
            execution_matrix_cm.push(g);
        }
        
        /*
        let mut last_group = None;
        let mut exec_counter = 0;
        
        while let Some(node) = topo.next(&graph) {
            if node == user || node == post_user {
                continue;
            }

            let (stage, _) = nodes.iter().find(|x| *x.1 == node).unwrap();

            let group = groups.iter().position(|x| x.3.contains(&node));
            if let Some(i) = group {
                if last_group == Some(i) {
                    execution_matrix_cm[exec_counter - 1].push(*stage);
                } else {
                    exec_counter += 1;
                    execution_matrix_cm.push(vec![*stage]);
                }

                last_group = Some(i);
            }

            log::debug!(
                "System: {}, group: {:?}",
                graph[node].name,
                group,
            );

            count += 1;
        }
        */

        // If there are missing nodes then we must have a cylic reference
        if count < temp_vec.len() {
            return Err(RegistrySortingError::GraphVisitMissingNodes);
        }
        
        // Handle thread task overflow here (basically leak extra tasks to a new group, repeat until done)
        // I know this is really ugly. Will fix later
        while execution_matrix_cm.iter().any(|x| x.len() > thread_count) {
            let group_index_extra = execution_matrix_cm.iter().position(|x| x.len() > thread_count).unwrap();
            log::debug!("Goup index: {group_index_extra}");
            let extras = execution_matrix_cm[group_index_extra].drain(thread_count..).collect::<Vec<_>>();

            if extras.is_empty() {
                panic!();
            }

            log::debug!("Extra count: {}", extras.len());
            execution_matrix_cm.insert(group_index_extra + 1, extras);
        }

        let mut ascii_table = AsciiTable::default();
        for (i, execs) in execution_matrix_cm.iter().enumerate() {
            ascii_table.column(i + 1).set_header(format!("{i}"));

            for j in 0..thread_count {
                if let Some(x) = execs.get(j) {
                    let name = &x.name.split("::").last().unwrap();
                    data[j].push(name.to_string());
                } else {
                    data[j].push("__".to_string());
                }
            }
        }
        log::debug!("\n{}", ascii_table.format(data));

        let per_thread = row_major(thread_count, &execution_matrix_cm, self.systems);
        
        Ok(SortedRegistry {
            column_major: execution_matrix_cm,
            per_thread
        })
    }       
}

fn row_major(thread_count: usize, execution_matrix_cm: &Vec<Vec<StageId>>, mut systems: AHashMap<StageId, Internal>) -> Vec<Vec<Option<Internal>>> {
    // must convert the column major data to row major so each thread has to worry about its own data only
    let mut per_thread = Vec::<Vec<Option<Internal>>>::default();
    for _ in 0..thread_count {
        per_thread.push(Vec::new());
    }
    
    for parallel in execution_matrix_cm {
        for i in 0..thread_count {
            let internal = parallel.get(i).map(|i| systems.remove(i).unwrap());
            per_thread[i].push(internal);
        }
    }
    
    per_thread.retain(|x| x.iter().any(|x| x.is_some()));
    let max = per_thread.iter().map(|x| x.len()).max().unwrap();
    for x in per_thread.iter_mut() {
        x.resize_with(max, || None);
    }
    per_thread
}

// calculate the depths of the systems starting from the user system
fn calculate_depths<'a>(graph: &Graph<StageId, ()>, user: NodeIndex) -> HashMap<NodeIndex, i32> {
    let mut new = UnGraph::default();
    for node in graph.node_indices() {
        new.add_node(graph[node]);
    }

    for edge in graph.edge_references() {
        let source = edge.source();
        let target = edge.target();    
        new.add_edge(source, target, ());
        new.add_edge(target, source, ());
    }

    let first = Topo::new(&graph).next(&graph).unwrap();

    return k_shortest_path(&new, first, None, 1, |edge| { 1
    });
}