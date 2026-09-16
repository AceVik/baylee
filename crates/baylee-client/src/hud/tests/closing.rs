use super::*;

#[test]
fn closing_the_duel_takes_the_overlay_with_it() {
    let mut app = App::new();
    app.init_resource::<HudRevision>()
        .init_resource::<LedgeRevision>()
        .init_resource::<crate::hud::LedgeLayout>()
        .init_resource::<crate::hud::DrawerRevision>()
        .add_systems(Update, despawn_overlay);
    let root = app.world_mut().spawn(HudRoot).id();
    let child = app.world_mut().spawn(Node::default()).id();
    app.world_mut().entity_mut(root).add_child(child);
    app.world_mut().resource_mut::<HudRevision>().seq = Some(7);
    app.world_mut().resource_mut::<LedgeRevision>().seq = Some(7);

    app.update();

    let mut roots = app.world_mut().query_filtered::<Entity, With<HudRoot>>();
    assert_eq!(roots.iter(app.world()).count(), 0, "the root is gone");
    let mut nodes = app.world_mut().query_filtered::<Entity, With<Node>>();
    assert_eq!(
        nodes.iter(app.world()).count(),
        0,
        "and its children went with it"
    );
    assert!(
        app.world().resource::<HudRevision>().seq.is_none(),
        "a revision describing a tree that no longer exists would make the \
         next duel's first frame skip its own rebuild"
    );
    assert!(
        app.world().resource::<LedgeRevision>().seq.is_none(),
        "and the shelf's own counter describes the same vanished tree: \
         its columns would be compared against a snapshot from the \
         previous duel and never built"
    );
}
