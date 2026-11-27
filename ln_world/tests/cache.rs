use ln_world::{Element, World};

#[derive(Debug, PartialEq, Eq)]
struct Tag(&'static str);
impl Element for Tag {}

#[test]
fn cache_invalidated_on_remove() {
    let mut world = World::default();

    let node1 = world.insert(Tag("node1"));
    let node2 = world.insert(Tag("node2"));
    world.flush();

    world.queue_cache::<Tag>();
    world.flush();
    assert_eq!(tags(&world), vec!["node1", "node2"]);

    world.remove(node2);
    world.flush();
    assert_eq!(tags(&world), vec!["node1"]);

    assert!(world.validate(node1).is_ok());
    assert!(world.validate(node2).is_err());
}

#[test]
fn cache_invalidated_on_insert() {
    let mut world = World::default();

    world.insert(Tag("first"));
    world.flush();

    world.queue_cache::<Tag>();
    world.flush();
    assert_eq!(tags(&world), vec!["first"]);

    world.insert(Tag("second"));
    world.flush();

    assert_eq!(tags(&world), vec!["first", "second"]);
}

fn tags(world: &World) -> Vec<&str> {
    let mut got = Vec::new();
    world.foreach_fetch::<Tag>(|h| got.push(h.0));
    got.sort();
    got
}
