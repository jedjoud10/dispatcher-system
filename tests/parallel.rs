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
fn full() {
    env_logger::Builder::from_default_env()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let mut registry = Registry::default();

    registry
        .insert(system_a)
        .unwrap()
        .reads::<ResA>()
        .writes::<ResB>();
    registry
        .insert(system_c)
        .unwrap()
        .reads::<ResA>()
        .writes::<ResC>();
    registry
        .insert(system_b)
        .unwrap()
        .writes::<ResB>()
        .writes::<ResC>()
        .after(system_d);
    registry.insert(system_d).unwrap().reads::<ResC>();
    registry
        .insert(system_e)
        .unwrap()
        .reads::<ResB>()
        .before(system_b)
        .after(system_a);

    let builder = registry.sort().unwrap();
    assert_eq!(
        builder.group(0),
        Some(&vec![StageId::of(&system_a), StageId::of(&system_c)])
    );
    assert_eq!(builder.group(1), Some(&vec![StageId::of(&system_d)]));
    assert_eq!(builder.group(2), Some(&vec![StageId::of(&system_e)]));
    assert_eq!(builder.group(3), Some(&vec![StageId::of(&system_b)]));
}

#[test]
fn simpler() {
    env_logger::Builder::from_default_env()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let mut registry = Registry::default();

    registry
        .insert(system_a)
        .unwrap()
        .reads::<ResA>()
        .writes::<ResB>();
    registry
        .insert(system_b)
        .unwrap()
        .reads::<ResC>()
        .writes::<ResC>();

    let builder = registry.sort().unwrap();
    assert_eq!(
        builder.group(0),
        Some(&vec![StageId::of(&system_a), StageId::of(&system_b)])
    );
}

#[test]
fn all() {
    env_logger::Builder::from_default_env()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let mut registry = Registry::default();

    registry.insert(system_a).unwrap();
    registry.insert(system_c).unwrap();
    registry.insert(system_b).unwrap();
    registry.insert(system_d).unwrap();
    registry.insert(system_e).unwrap();

    let builder = registry.sort().unwrap();
    assert_eq!(builder.group(0).unwrap().len(), 5);
}

#[test]
fn balancing() {
    env_logger::Builder::from_default_env()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let mut registry = Registry::default();

    registry.insert(system_a).unwrap();
    registry.insert(system_c).unwrap();
    registry.insert(system_b).unwrap();
    registry.insert(system_d).unwrap();
    registry.insert(system_e).unwrap();

    let mut builder = registry.sort().unwrap();
    builder.balance(Some(2));
    assert_eq!(builder.group(0).unwrap().len(), 2);
    assert_eq!(builder.group(1).unwrap().len(), 2);
    assert_eq!(builder.group(2).unwrap().len(), 1);
}
