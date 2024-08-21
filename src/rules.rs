use crate::{stage::StageId, world::World};

// A rule that depicts the arrangement and the location of the stages relative to other stages
#[derive(Clone, Debug, Hash)]
pub enum InjectionRule {
    Before(StageId),
    After(StageId),

    // do note that in some cases where the threads are all saturated with tasks,
    // the registry will sort fine even though the underlying tasks will NOT run in parallel
    Parallel(StageId),
}

pub fn user(_: &World) {}

pub fn post_user(_: &World) {}

// Create the default rules for a default node
pub(super) fn default_rules() -> Vec<InjectionRule> {
    let after = InjectionRule::After(StageId::of(&user));
    let before = InjectionRule::Before(StageId::of(&post_user));
    vec![before, after]
}
