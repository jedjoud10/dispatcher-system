use crate::{stage::StageId, world::World};

// A rule that depicts the arrangement and the location of the stages relative to other stages
#[derive(Clone, Debug)]
pub enum InjectionRule {
    // This hints that the stage should be executed before other
    Before(StageId),

    // This hints that the stage should be executed after other
    After(StageId),

    // This tells the dispatcher to execute this system in parallel with another system
    // This should automatically be handled by the graph system but this is to FORCE such a system to be executed like that
    Parallel(StageId),
}

impl InjectionRule {
    // Get the node this rule is referencing
    pub(super) fn reference(&self) -> StageId {
        match self {
            InjectionRule::Before(p) => *p,
            InjectionRule::After(p) => *p,
            InjectionRule::Parallel(p) => panic!("Don't make sense"),
        }
    }
}

pub fn user(world: &World) {
}

pub fn post_user(world: &World) {
}

// Create the default rules for a default node
pub(super) fn default_rules() -> Vec<InjectionRule> {
    let after = InjectionRule::After(StageId::fetch(&user));
    let before = InjectionRule::Before(StageId::fetch(&post_user));
    vec![before, after]
}