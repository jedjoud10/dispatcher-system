use std::marker::PhantomData;
use crate::{inject::InjectionOrder, world::World};

#[derive(Default)]
pub struct IncompleteRegistry<E> {
    _phantom: PhantomData<E>,
}

impl<E> IncompleteRegistry<E> {
    pub fn insert<S: FnMut(&World)>(&mut self, mut system: S) -> InjectionOrder<E> {
        todo!()
    }

    pub fn sort(&mut self) {
    }

    pub fn execute(&mut self) {
    }
}