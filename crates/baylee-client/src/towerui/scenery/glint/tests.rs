use super::*;

#[test]
fn cabinet_and_frost_share_cached_finishes_and_reduced_motion_never_uploads_at_idle() {
    let mut app = App::new();
    app.init_resource::<Cached>()
        .init_resource::<Assets<Finish>>()
        .init_resource::<crate::prefs::Prefs>()
        .add_systems(Update, dress);
    for _ in 0..3 {
        let frame = app.world_mut().spawn((Node::default(), Decor::Frame)).id();
        let frost = app.world_mut().spawn((Node::default(), FrostSurface)).id();
        app.update();
        let world = app.world_mut();
        assert_eq!(world.resource::<Assets<Finish>>().len(), 2);
        assert_eq!(
            world.query::<&MaterialNode<Finish>>().iter(world).count(),
            2
        );
        world.despawn(frame);
        world.despawn(frost);
    }
    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .reduce_motion = true;
    app.update();
    app.world_mut().spawn((Node::default(), Decor::Frame));
    app.world_mut().spawn((Node::default(), FrostSurface));
    app.update();
    assert!(
        app.world()
            .resource::<Assets<Finish>>()
            .iter()
            .all(|(_, finish)| finish.motion.x.abs() < f32::EPSILON)
    );
    let tick = app.world().read_change_tick();
    app.update();
    assert!(
        !app.world()
            .resource_ref::<Assets<Finish>>()
            .last_changed()
            .is_newer_than(tick, app.world().read_change_tick())
    );
    app.world_mut()
        .resource_mut::<crate::prefs::Prefs>()
        .edit()
        .reduce_motion = false;
    app.update();
    assert!(
        app.world()
            .resource::<Assets<Finish>>()
            .iter()
            .all(|(_, finish)| (finish.motion.x - 1.0).abs() < f32::EPSILON)
    );
}

#[test]
fn cabinet_shader_validates_with_native_frontend() {
    crate::cardmat::tests::check_wgsl(
        include_str!("../glint.wesl"),
        "struct UiVertexOutput { @location(0) uv: vec2<f32>, @location(1) size: vec2<f32>, @builtin(position) position: vec4<f32> }; struct Globals { time: f32 };",
    );
}
