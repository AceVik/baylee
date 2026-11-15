use super::*;

#[test]
fn rebuilt_portraits_share_materials_and_idle_frames_never_upload() {
    let mut app = App::new();
    app.init_resource::<Assets<Material>>()
        .init_resource::<Cache>()
        .add_systems(Update, dress);
    for _ in 0..3 {
        let entities: Vec<_> = [
            Family::Heartwood,
            Family::Satchel,
            Family::Compass,
            Family::Grove,
        ]
        .into_iter()
        .flat_map(|family| {
            [
                Rarity::Common,
                Rarity::Uncommon,
                Rarity::Rare,
                Rarity::MythicRare,
            ]
            .map(|rarity| Portrait(Goods::Relic(Definition { family, rarity })))
        })
        .map(|portrait| app.world_mut().spawn(portrait).id())
        .collect();
        app.update();
        assert_eq!(app.world().resource::<Assets<Material>>().len(), 16);
        for entity in entities {
            assert!(app.world().get::<MaterialNode<Material>>(entity).is_some());
            app.world_mut().despawn(entity);
        }
    }
    app.update();
    let tick = app.world().read_change_tick();
    app.update();
    assert!(
        !app.world()
            .resource_ref::<Assets<Material>>()
            .last_changed()
            .is_newer_than(tick, app.world().read_change_tick())
    );
}

#[test]
fn item_shader_validates_with_native_frontend() {
    crate::cardmat::tests::check_wgsl(
        include_str!("../art.wesl"),
        "struct UiVertexOutput { @location(0) uv: vec2<f32>, @builtin(position) position: vec4<f32> }; struct Globals { time: f32 };",
    );
}
