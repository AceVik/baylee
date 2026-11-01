use super::*;
use bevy::ecs::system::RunSystemOnce;

#[test]
fn first_frame_placement_preserves_the_menus_arrival_offset() {
    let mut world = World::new();
    world.insert_resource(LobbyState::default());
    world.init_resource::<MenuMemory>();
    world.spawn(Window::default());
    let anchor = world
        .spawn((
            ComputedNode {
                size: Vec2::new(100.0, 40.0),
                ..default()
            },
            UiGlobalTransform::from_translation(Vec2::new(200.0, 80.0)),
        ))
        .id();
    let menu = world
        .spawn((
            MenuAnchor(anchor),
            Unplaced,
            Opening::default(),
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                ..default()
            },
            ComputedNode {
                size: Vec2::new(200.0, 100.0),
                ..default()
            },
            UiTransform::from_translation(Val2::px(0.0, -5.0)),
            UiGlobalTransform::from_translation(Vec2::new(100.0, 45.0)),
        ))
        .id();
    world.run_system_once(place).unwrap();
    let min = world.get::<UiGlobalTransform>(menu).unwrap().translation - Vec2::new(100.0, 50.0);
    assert!(
        (min.y - 99.0).abs() < 0.01,
        "first-frame y was {min:?}, expected layout top 104 minus arrival offset 5"
    );
}
