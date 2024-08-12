use ahash::AHashMap;
use ascii_table::AsciiTable;
use petgraph::{algo::k_shortest_path, graph::{NodeIndex, UnGraph}, visit::{EdgeRef, Topo}, Graph};
use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::Arc,
};

use crate::{
    dispatcher::Dispatcher, inject::InjectionOrder, rules::{default_rules, post_user, user, InjectionRule}, stage::StageId, world::World, RegistrySortingError, ResourceMask, StageError
};

pub(crate) struct Internal {
    pub(crate) boxed: Box<dyn FnMut(&World) + Sync + Send>,
    pub(crate) rules: Vec<InjectionRule>,
    pub(crate) reads: ResourceMask,
    pub(crate) writes: ResourceMask,
}

#[derive(Default)]
pub struct UnfinishedRegistry<E> {
    _phantom: PhantomData<E>,
    systems: AHashMap<StageId, Internal>,
    thread_count: Option<usize>,
}

impl<E> UnfinishedRegistry<E> {
    pub fn insert<S: FnMut(&World) + Sync + Send + 'static>(
        &mut self,
        system: S,
    ) -> Result<InjectionOrder<E>, StageError> {
        let rules = default_rules();
        let stage = StageId::fetch(&system);
        
        if self.systems.contains_key(&stage) {
            return Err(StageError::Overlapping);
        }

        if [StageId::fetch(&user), StageId::fetch(&post_user)].contains(&stage) {
            return Err(StageError::InvalidName);
        }

        self.systems.insert(
            stage,
            Internal {
                boxed: Box::new(system),
                rules,
                reads: ResourceMask::default(),
                writes: ResourceMask::default(),
            },
        );
        let internal = self.systems.get_mut(&stage).unwrap();
        Ok(InjectionOrder::new(internal))
    }

    pub fn set_thread_count(&mut self, thread_count: Option<usize>) {
        self.thread_count = thread_count;
    }

    // two (three) constraints
    // 1) make sure ordering constraint is held (sys a before sys b)
    // 2) make sure no intersecting RW masks
    // 3) (optional) optimize RW masks to improve concurrency
    pub fn sort(mut self, world: Arc<World>) -> Result<Dispatcher, RegistrySortingError> {
        let thread_count = self.thread_count.unwrap_or_else(num_cpus::get).max(1);
        let mut graph = Graph::<StageId, &InjectionRule>::new();

        let mut temp_vec = self.systems.iter().collect::<Vec<_>>();
        temp_vec.sort_by_key(|x| x.0);
        let mut nodes = temp_vec
            .iter()
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
                let reference = *nodes.get(&reference).ok_or(RegistrySortingError::MissingStage(**node, reference))?;

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
        dbg!(graph.node_count());
        let copy = graph.clone();
        let mut undirected_graph = UnGraph::<StageId, &InjectionRule>::new_undirected();

        for node in graph.node_indices() {
            undirected_graph.add_node(graph[node]);
        }

        for edge in graph.edge_references() {
            let source = edge.source();
            let target = edge.target();
            let weight = edge.weight();
    
            undirected_graph.add_edge(source, target, weight);
            undirected_graph.add_edge(target, source, weight);
        }

        let graph = undirected_graph;
        let path = k_shortest_path(&graph, user, None, 1, |_| 1);

        for (node, index) in &path {
            log::debug!("System: {}, Index: {}", graph.node_weight(*node).unwrap().name, index);
        }

        let graph = copy;

        let mut path_sorted = path
            .iter()
            .map(|(idx, temp)| {
                (
                    idx,
                    temp,
                    self.systems
                        .get(&graph[*idx])
                        .map(|a| a.reads.count_ones() + a.writes.count_ones())
                        .unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>();

        // TODO: Sort the graph to minimize system fragmentation so that we can merge as many systems as possible together
        // Must apply some "weight" to the nodes so that nodes WITHOUT dependants are executed first (if dependency tree allows)
        // This way we can execute all similar systems first, then we can start subdividing down

        // FIX: Easiest fix would be to sort the systems based on rule set (so systems with similar rules are together)
        // unfortunately this system shits itself when the user defines more rules than necessary (as that would change their hash and ordering n shit)
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

        // this will batch the system into their "group" batches where they can execute in parallel with other systems
        // without overlapping their resource bitmasks
        for (&index, &depth, _) in path_sorted.iter() {
            let (stage_id, _) = nodes.iter().find(|x| *x.1 == index).unwrap();

            let Some(internal) = self.systems.get(stage_id) else {
                continue;
            };

            let node_reads = internal.reads;
            let node_writes = internal.writes;
            log::debug!(
                "System: {} Depth: {} R: {:#06b}, W: {:#06b}",
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
                        let deptho = *group_depth == depth;

                        // check for ref-mut collisions
                        let ref_mut_collisions = (node_reads | node_writes) & group_writes == 0
                            && ((node_writes) & group_reads == 0);

                        // check for mut-mut collisions
                        let mut_mut_collisions = (node_writes & group_writes) == 0;

                        deptho && ref_mut_collisions && mut_mut_collisions
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
        let mut topo = Topo::new(&graph);
        let mut data = Vec::<Vec<String>>::default();
        for i in 0..thread_count {
            let a = format!("Thread: {}", i + 1);
            data.push(vec![a]);
        }

        let mut last_group = None;
        let mut exec_counter = 0;

        // column based table to know what to execute in parallel
        let mut execution_matrix_cm = Vec::<Vec<StageId>>::default();
        let mut count = 0;
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
                "System: {}, group: {:?}, depth: {}",
                graph[node].name,
                group,
                path[&node]
            );

            count += 1;
        }
        
        // Handle thread task overflow here (basically leak extra tasks to a new group, repeat until done)
        // I know this is really ugly. Will fix later
        while execution_matrix_cm.iter().any(|x| x.len() > thread_count) {
            let group_index_extra = execution_matrix_cm.iter().position(|x| x.len() > thread_count).unwrap();
            log::debug!("Goup index: {group_index_extra}");
            let extras = execution_matrix_cm[group_index_extra].drain(thread_count..).collect::<Vec<_>>();

            if extras.is_empty() {
                panic!("fuckj");
            }

            log::debug!("Extra count: {}", extras.len());
            execution_matrix_cm.insert(group_index_extra + 1, extras);
        }
        
        // If there are missing nodes then we must have a cylic reference
        if count < temp_vec.len() {
            return Err(RegistrySortingError::GraphVisitMissingNodes);
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

        // must convert the column major data to row major so each thread has to worry about its own data only
        let mut per_thread = Vec::<Vec<Option<Internal>>>::default();
        for _ in 0..thread_count {
            per_thread.push(Vec::new());
        }

        for parallel in execution_matrix_cm {
            for i in 0..thread_count {
                let internal = parallel.get(i).map(|i| self.systems.remove(i).unwrap());
                per_thread[i].push(internal);
            }

            /*
            // we should try splitting all the parallel tasks evenly across multiple threads
            // could even implement a "priority" system where one thread takes most of the tasks or if the user wants a task to execute on a specific 
            let mut thread_index = 0;
            while !parallel.is_empty() {
                thread_index %= thread_count; 
                let removed = parallel.remove(0); 
                
                let internal = self.systems.remove(&removed);
                per_thread[thread_index].push(internal);
                thread_index += 1;
            }
            */
        }        

        per_thread.retain(|x| x.iter().any(|x| x.is_some()));
        let max = per_thread.iter().map(|x| x.len()).max().unwrap();

        for x in per_thread.iter_mut() {
            x.resize_with(max, || None);
        }

        Ok(Dispatcher::build(per_thread, world))
    }
}
