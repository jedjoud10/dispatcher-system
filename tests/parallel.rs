use dispatcher_system::*;

struct ResA;
struct ResB;
struct ResC;

fn system_a(_: &World) {}
fn system_b(_: &World) {}
fn system_c(_: &World) {}
fn system_d(_: &World) {}
fn system_e(_: &World) {}

#[test]
fn main() {
    env_logger::Builder::from_default_env().is_test(true).filter_level(log::LevelFilter::Debug).init();

    let mut registry = UnfinishedRegistry::default();
    
    registry.insert(system_a).unwrap().reads::<ResA>().writes::<ResB>();
    registry.insert(system_c).unwrap().writes::<ResC>().reads::<ResA>();
    registry.insert(system_b).unwrap().writes::<ResB>().writes::<ResC>().after(system_d);
    registry.insert(system_d).unwrap().reads::<ResC>();
    registry.insert(system_e).unwrap().reads::<ResB>().before(system_b).after(system_a);
    
    let sorted = registry.sort().unwrap();
    assert_eq!(sorted.group(0), Some(&vec![StageId::of(&system_a), StageId::of(&system_c)]));
    assert_eq!(sorted.group(1), Some(&vec![StageId::of(&system_d)]));
    assert_eq!(sorted.group(2), Some(&vec![StageId::of(&system_e)]));
    assert_eq!(sorted.group(3), Some(&vec![StageId::of(&system_b)]));
}
