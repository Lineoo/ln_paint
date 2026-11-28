use lnworld::{Element, World};

#[derive(Debug, PartialEq, Eq)]
struct Tag(&'static str);
impl Element for Tag {}

struct Tick;

/// An observer must die together with its target.
#[test]
fn observer_removed_with_target() {
    let mut world = World::default();

    let target = world.insert(Tag("t"));
    world.flush();

    let observer = world.observer(target, move |_: &Tick, _: &World| {});
    world.flush();

    world.remove(target);
    world.flush();

    assert!(world.validate(observer).is_err());
}
