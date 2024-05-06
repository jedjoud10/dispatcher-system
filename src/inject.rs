use std::{marker::PhantomData, any::TypeId};

use crate::{resources::ResourceMask, world::World};

pub struct InjectionOrder<'a, E> {
    pub(crate) rules: &'a mut Vec<InjectionRule>,
    pub(crate) default: bool,
    pub(crate) _phantom: PhantomData<E>,
}

#[derive(Clone, Debug)]
pub enum InjectionRule {
    Before(TypeId),
    After(TypeId),
}

impl<'a, E> InjectionOrder<'a, E> {
    pub fn writes(mut self, mask: ResourceMask) -> Self {
        self
    }

    pub fn reads(mut self, mask: ResourceMask) -> Self {
        self
    }

    fn reset_defaults(&mut self) {
        if std::mem::take(&mut self.default) {
            self.rules.clear();
        }
    }

    pub fn before<S: FnMut(&World) + 'static>(mut self, system: S) -> Self {
        self.reset_defaults();
        self.rules.push(InjectionRule::Before(TypeId::of::<S>()));
        self
    }

    pub fn after<S: FnMut(&World) + 'static>(mut self, system: S) -> Self {
        self.reset_defaults();
        self.rules.push(InjectionRule::After(TypeId::of::<S>()));
        self
    }
}

pub fn pre_user<E>(_: &mut World, _: &E) {}
pub fn post_user<E>(_: &mut World, _: &E) {}
