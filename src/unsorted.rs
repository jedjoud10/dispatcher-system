use ahash::AHashMap;
use petgraph::{
    graph::NodeIndex,
    visit::{EdgeRef, Topo},
    Graph,
};

use crate::{
    inject::InjectionOrder,
    rules::{default_rules, post_user, user, InjectionRule},
    stage::StageId,
    world::World,
    DispatchBuilder, RegistrySortingError, ResourceMask, StageError,
};

pub(crate) struct Internal {
    pub(crate) boxed: Box<dyn FnMut(&World) + Sync + Send>,
    pub(crate) rules: Vec<InjectionRule>,
    pub(crate) reads: ResourceMask,
    pub(crate) writes: ResourceMask,
}

#[derive(Default)]
pub struct Registry {
    systems: AHashMap<StageId, Internal>,
}

impl Registry {
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
                boxed: Box::new(system),
                rules,
                reads: ResourceMask::default(),
                writes: ResourceMask::default(),
            },
        );
        let internal = self.systems.get_mut(&stage).unwrap();
        Ok(InjectionOrder::new(internal))
    }

    // two (three) constraints
    // 1) make sure ordering constraint is held (sys a before sys b)
    // 2) make sure no intersecting RW masks
    // 3) (optional) optimize RW masks to improve concurrency
    pub fn sort(self) -> Result<DispatchBuilder, RegistrySortingError> {
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

        let mut should_execute_in_parallel = Vec::<Vec<StageId>>::new();

        for (node, internal) in temp_vec.iter() {
            for rule in internal.rules.iter() {
                let this = nodes[node];
                let reference = match rule {
                    InjectionRule::Before(p) => *p,
                    InjectionRule::After(p) => *p,
                    InjectionRule::Parallel(p) => *p,
                };
                
                let reference_node = *nodes
                    .get(&reference)
                    .ok_or(RegistrySortingError::MissingStage(**node, reference))?;

                match rule {
                    // dir: a -> b.
                    // dir: this -> reference
                    InjectionRule::Before(_) => { graph.add_edge(this, reference_node, ()); },

                    // dir: a -> b.
                    // dir: reference -> this
                    InjectionRule::After(_) => { graph.add_edge(reference_node, this, ()); },

                    // find a rule group that we can add the stage id into
                    InjectionRule::Parallel(_) => {
                        let group = should_execute_in_parallel.iter_mut().find(|x| x.contains(&reference));
                        if let Some(group) = group {
                            group.push(**node);
                        } else {
                            should_execute_in_parallel.push(vec![**node, reference]);
                        }
                    },
                };
            }
        }

        // Groups for each type of resource access
        // Must correspond to the "depth" of each nodes as we can't mix and match groups from different levels (otherwise it would fuck
        // with the first requirement of having proper depedency sorting)
        let mut groups = Vec::<(i32, ResourceMask, ResourceMask, Vec<NodeIndex>)>::default();

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

            log::debug!(
                "{:?}, Depth: {:?}",
                graph.node_weight(a).unwrap(),
                test.get(&a)
            );
            if test.get(&a).is_some() {
                for x in graph.edges_directed(a, petgraph::Direction::Outgoing) {
                    let rizz = test
                        .get(&a)
                        .map(|x| match x {
                            Testino::Concrete(u) => Testino::Concrete(u + 1),
                            Testino::Ref(u) => Testino::Ref(*u),
                        })
                        .unwrap_or_else(|| Testino::Ref(a));

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

            // TODO: one could even write a custom heuristic function here to split off systems within groups if needed
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

        // column based table to know what to execute in parallel
        let mut execution_matrix_cm = Vec::<Vec<StageId>>::default();
        let mut count = 0;
        groups.sort_by_key(|(depth, _, _, _)| *depth);
        for (_, _, _, x) in groups.iter() {
            let g = x
                .iter()
                .map(|a| *graph.node_weight(*a).unwrap())
                .collect::<Vec<_>>();
            count += g.len();
            execution_matrix_cm.push(g);
        }

        // Check for parallel rules to make sure we upheld them
        let mut fine = true;
        for a in should_execute_in_parallel {
            fine &= execution_matrix_cm.iter().any(|y| a.iter().all(|j| y.contains(j)));
        }

        if !fine {
            return Err(RegistrySortingError::UnsatisfiableParallelRules);
        }

        // If there are missing nodes then we must have a cylic reference
        if count < temp_vec.len() {
            return Err(RegistrySortingError::GraphVisitMissingNodes);
        }

        Ok(DispatchBuilder {
            execution_matrix_cm,
            systems: self.systems,
            per_thread: Default::default(),
            balanced_thread_count: 0,
        })
    }
}
