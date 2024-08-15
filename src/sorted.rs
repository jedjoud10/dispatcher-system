use std::sync::Arc;

use ahash::AHashMap;
use ascii_table::AsciiTable;

use crate::{Dispatcher, Internal, StageId, World};

pub struct DispatchBuilder {
    pub(crate) execution_matrix_cm: Vec<Vec<StageId>>,
    pub(crate) systems: AHashMap<StageId, Internal>,
    pub(crate) per_thread: Vec<Vec<Option<Internal>>>,
    pub(crate) balanced_thread_count: usize,
}

impl DispatchBuilder {
    pub fn balance(&mut self, thread_count: Option<usize>) {
        let thread_count = thread_count.unwrap_or_else(|| num_cpus::get() - 1).max(1);

        // Handle thread task overflow here (basically leak extra tasks to a new group, repeat until done)
        // I know this is really ugly. Will fix later
        let execution_matrix_cm = &mut self.execution_matrix_cm;
        while execution_matrix_cm.iter().any(|x| x.len() > thread_count) {
            let group_index_extra = execution_matrix_cm
                .iter()
                .position(|x| x.len() > thread_count)
                .unwrap();
            log::debug!("Goup index: {group_index_extra}");
            let extras = execution_matrix_cm[group_index_extra]
                .drain(thread_count..)
                .collect::<Vec<_>>();

            if extras.is_empty() {
                panic!();
            }

            log::debug!("Extra count: {}", extras.len());
            execution_matrix_cm.insert(group_index_extra + 1, extras);
        }

        let per_thread = row_major(
            thread_count,
            execution_matrix_cm,
            std::mem::take(&mut self.systems),
        );
        self.per_thread = per_thread;
        self.balanced_thread_count = thread_count;
    }

    pub fn build(mut self, world: Arc<World>, thread_count: Option<usize>) -> Dispatcher {
        if self.per_thread.is_empty() {
            self.balance(thread_count);
        }

        let mut data = Vec::<Vec<String>>::default();
        let thread_count = self.balanced_thread_count;
        for i in 0..thread_count {
            let a = format!("Thread: {}", i + 1);
            data.push(vec![a]);
        }

        let mut ascii_table = AsciiTable::default();
        for (i, execs) in self.execution_matrix_cm.iter().enumerate() {
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

        Dispatcher::build(self.per_thread, world)
    }

    pub fn group(&self, group: usize) -> Option<&Vec<StageId>> {
        self.execution_matrix_cm.get(group)
    }

    pub fn stage_at(&self, group: usize, thread: usize) -> Option<StageId> {
        let group = self.execution_matrix_cm.get(group)?;
        group.get(thread).copied()
    }
}

fn row_major(
    thread_count: usize,
    execution_matrix_cm: &Vec<Vec<StageId>>,
    mut systems: AHashMap<StageId, Internal>,
) -> Vec<Vec<Option<Internal>>> {
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
