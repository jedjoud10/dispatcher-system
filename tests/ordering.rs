#![allow(unused_must_use)]
use dispatcher_system::*;

fn system_a(_: &World) {}
fn system_b(_: &World) {}
fn system_c(_: &World) {}
fn system_d(_: &World) {}
fn system_e(_: &World) {}
fn system_f(_: &World) {}

#[test]
fn main() {
    env_logger::Builder::from_default_env()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let mut registry = Registry::default();

    registry.insert(system_e).unwrap().before(system_d);

    registry
        .insert(system_d)
        .unwrap()
        .before(post_user)
        .before(user);

    registry.insert(system_a).unwrap();
    registry
        .insert(system_c)
        .unwrap()
        .before(post_user)
        .after(user)
        .parallel(system_c);

    registry
        .insert(system_b)
        .unwrap()
        .after(post_user)
        .after(user);

    registry.insert(system_f).unwrap().after(system_b);

    let builder = registry.sort().unwrap();
    assert_eq!(builder.group(0), Some(&vec![StageId::of(&system_e)]));
    assert_eq!(builder.group(1), Some(&vec![StageId::of(&system_d)]));
    assert_eq!(
        builder.group(2),
        Some(&vec![StageId::of(&system_a), StageId::of(&system_c)])
    );
    assert_eq!(builder.group(3), Some(&vec![StageId::of(&system_b)]));
    assert_eq!(builder.group(4), Some(&vec![StageId::of(&system_f)]));
}
