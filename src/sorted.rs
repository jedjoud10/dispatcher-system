use std::sync::Arc;

use crate::{Dispatcher, Internal, StageId, World};

pub struct SortedRegistry {
    pub(crate) column_major: Vec<Vec<StageId>>,
    pub(crate) per_thread: Vec<Vec<Option<Internal>>>,
}

impl SortedRegistry {
    pub fn build(self, world: Arc<World>) -> Dispatcher {
        Dispatcher::build(self.per_thread, world)
    }

    pub fn group(&self, group: usize) -> Option<&Vec<StageId>> {
        self.column_major.get(group)
    }

    pub fn stage_at(&self, group: usize, thread: usize) -> Option<StageId> {
        let group = self.column_major.get(group)?;
        group.get(thread).map(|x| *x)
    }
}
