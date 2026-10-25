use super::*;

#[test]
fn a_removed_chat_link_dismisses_its_preview() {
    let mut app = App::new();
    app.init_resource::<Duel>()
        .init_resource::<ChatHover>()
        .add_message::<bevy::picking::events::PointerOver>()
        .add_message::<bevy::picking::events::PointerOut>()
        .add_systems(Update, hover);
    let span = app
        .world_mut()
        .spawn(rows::ChatCardLink {
            card: 7,
            face: 0,
            print: None,
        })
        .id();
    app.world_mut().resource_mut::<ChatHover>().0 = Some(Request {
        span,
        identity: CardIdentity {
            index: baylee_core::ids::CardIndex::new(7),
            print: baylee_core::ids::PrintRef::new(0),
            face: 0,
        },
        at: Vec2::ZERO,
    });
    app.update();
    assert!(app.world().resource::<ChatHover>().0.is_some());
    app.world_mut().entity_mut(span).despawn();
    app.update();
    assert!(
        app.world().resource::<ChatHover>().0.is_none(),
        "a remounted/removed chat row left its preview behind"
    );
}
