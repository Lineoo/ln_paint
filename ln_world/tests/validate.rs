use lnworld::{Element, World};

#[derive(Debug, PartialEq, Eq)]
struct Tag(&'static str);
impl Element for Tag {}

/// Guard: cached iteration must not observe elements that were removed but not
/// yet flushed.
#[test]
fn cache_does_not_see_just_removed_before_flush() {
    let mut world = World::default();

    let first = world.insert(Tag("first"));
    world.insert(Tag("second"));
    world.flush();

    world.queue_cache::<Tag>();
    world.flush();

    world.remove(first);

    assert_eq!(tags(&world), vec!["second"]);

    world.flush();

    assert_eq!(tags(&world), vec!["second"]);
}

/// Guard: `single_fetch` must not hand out a live reference to a removed element.
#[test]
fn single_fetch_returns_just_removed_before_flush() {
    let mut world = World::default();

    let tag = world.insert(Tag("first"));
    world.flush();

    world.queue_cache::<Tag>();
    world.flush();

    world.remove(tag);

    assert!(world.try_single_fetch::<Tag>().is_err());
}

fn tags(world: &World) -> Vec<&str> {
    let mut got = Vec::new();
    world.foreach_fetch::<Tag>(|h| got.push(h.0));
    got.sort();
    got
}
