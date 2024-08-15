use dispatcher_system::*;

fn system_a(_: &World) {}
fn system_b(_: &World) {}

#[test]
fn main() {
    env_logger::Builder::from_default_env().is_test(true).filter_level(log::LevelFilter::Debug).try_init();

    let mut registry = UnfinishedRegistry::default();
    
    registry.insert(system_a).unwrap().before(user);
    registry.insert(system_b).unwrap().after(user);

    let sorted = registry.sort().unwrap();
    assert_eq!(sorted.group(0), Some(&vec![StageId::of(&system_a)]));
    assert_eq!(sorted.group(1), Some(&vec![StageId::of(&system_b)]));
}
