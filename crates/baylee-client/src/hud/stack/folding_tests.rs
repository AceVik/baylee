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
