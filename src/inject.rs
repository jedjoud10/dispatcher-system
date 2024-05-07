use std::{marker::PhantomData, any::TypeId};
use crate::{resources::ResourceMask, rules::InjectionRule, stage::StageId, world::World, Internal};

pub struct InjectionOrder<'a, E> {
    pub(crate) internal: &'a mut Internal,
    pub(crate) default: bool,
    pub(crate) _phantom: PhantomData<E>,
}

impl<'a, E> InjectionOrder<'a, E> {
    pub(crate) fn new(internal: &'a mut Internal) -> Self {
        Self {
            internal,
            default: true,
            _phantom: PhantomData,
        }
    }

    pub fn writes(mut self, mask: ResourceMask) -> Self {
        self.internal.writes |= mask;
        self
    }

    pub fn reads(mut self, mask: ResourceMask) -> Self {
        self.internal.reads |= mask;
        self
    }

    fn reset_defaults(&mut self) {
        if std::mem::take(&mut self.default) {
            self.internal.rules.clear();
        }
    }

    pub fn before<S: FnMut(&World) + 'static>(mut self, system: S) -> Self {
        self.reset_defaults();
        self.internal.rules.push(InjectionRule::Before(StageId::fetch(&system)));
        self
    }

    pub fn after<S: FnMut(&World) + 'static>(mut self, system: S) -> Self {
        self.reset_defaults();
        self.internal.rules.push(InjectionRule::After(StageId::fetch(&system)));
        self
    }
}