use super::*;
#[test]
fn folding_survives_rebuilds_and_reopens() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<StackFold>()
        .add_systems(Update, fold_the_stack);
    let body = app.world_mut().spawn((StackViewport, Node::default())).id();
    app.world_mut().resource_mut::<StackFold>().collapsed = true;
    let frame = |app: &mut App| {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.016));
        app.update();
    };
    frame(&mut app);
    let progress = app.world().resource::<StackFold>().open;
    assert!(progress > 0.0 && progress < 1.0);
    app.world_mut().entity_mut(body).despawn();
    let rebuilt = app.world_mut().spawn((StackViewport, Node::default())).id();
    frame(&mut app);
    assert!(app.world().resource::<StackFold>().open < progress);
    for _ in 0..40 {
        frame(&mut app);
    }
    assert_eq!(
        *app.world().get::<Visibility>(rebuilt).unwrap(),
        Visibility::Hidden
    );
    app.world_mut().resource_mut::<StackFold>().collapsed = false;
    frame(&mut app);
    assert_eq!(
        *app.world().get::<Visibility>(rebuilt).unwrap(),
        Visibility::Inherited
    );
}

#[test]
fn stack_group_expands_and_collapses_smoothly_across_rebuilds() {
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<crate::prefs::Prefs>()
        .init_resource::<StackFold>()
        .add_systems(Update, fold_the_stack);
    let group_id = ObjectId::new(42, 0);
    let drawer = app
        .world_mut()
        .spawn((
            StackGroupBody { group_id, rows: 2 },
            Node::default(),
            Visibility::Hidden,
        ))
        .id();
    let label = app
        .world_mut()
        .spawn((
            StackGroupToggleText { group_id },
            Text::new(glyph::CHEVRON_DOWN.to_string()),
        ))
        .id();
    let frame = |app: &mut App| {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.016));
        app.update();
    };
    frame(&mut app);
    assert_eq!(
        *app.world().get::<Visibility>(drawer).unwrap(),
        Visibility::Hidden
    );
    app.world_mut()
        .resource_mut::<StackFold>()
        .toggle_group(group_id);
    frame(&mut app);
    let open = app.world().resource::<StackFold>().group_open(group_id);
    assert!(open > 0.0 && open < 1.0);
    assert_eq!(
        *app.world().get::<Visibility>(drawer).unwrap(),
        Visibility::Inherited
    );
    assert_eq!(
        app.world().get::<Text>(label).unwrap().0,
        glyph::CHEVRON_UP.to_string()
    );
    for _ in 0..40 {
        frame(&mut app);
    }
    assert!((app.world().resource::<StackFold>().group_open(group_id) - 1.0).abs() < 1e-4);
    app.world_mut()
        .resource_mut::<StackFold>()
        .toggle_group(group_id);
    for _ in 0..40 {
        frame(&mut app);
    }
    assert_eq!(
        *app.world().get::<Visibility>(drawer).unwrap(),
        Visibility::Hidden
    );
}
