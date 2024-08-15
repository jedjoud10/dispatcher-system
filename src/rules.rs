use crate::{stage::StageId, world::World};

// A rule that depicts the arrangement and the location of the stages relative to other stages
#[derive(Clone, Debug, Hash)]
pub enum InjectionRule {
    // This hints that the stage should be executed before other
    Before(StageId),

    // This hints that the stage should be executed after other
    After(StageId),
}

impl InjectionRule {
    // Get the node this rule is referencing
    pub(super) fn reference(&self) -> StageId {
        match self {
            InjectionRule::Before(p) => *p,
            InjectionRule::After(p) => *p,
        }
    }
}

pub fn user(_: &World) {}

pub fn post_user(_: &World) {}

// Create the default rules for a default node
pub(super) fn default_rules() -> Vec<InjectionRule> {
    let after = InjectionRule::After(StageId::of(&user));
    let before = InjectionRule::Before(StageId::of(&post_user));
    vec![before, after]
}
