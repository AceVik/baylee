use super::*;

#[test]
fn the_bottom_docked_chat_resizes_from_its_top_right_corner() {
    let mut app = App::new();
    app.init_resource::<LobbyChatSizing>()
        .insert_resource(ChatUi {
            open: true,
            minimised: false,
            ..default()
        })
        .init_resource::<crate::social::zoom::Zoom>()
        .add_message::<PointerDrag>()
        .add_message::<PointerDragEnd>()
        .add_systems(Update, resize);
    let handle = handle(&mut app.world_mut().commands());
    app.world_mut().flush();
    let node = app.world().get::<Node>(handle).unwrap();
    assert_eq!(node.top, zpx(0.0));
    assert_eq!(node.bottom, Val::Auto);
    let start = lobby_panel_size(None, Vec2::new(1280.0, 720.0), 1.0);
    let event = crate::test_pointer::aim(
        PointerId::Mouse,
        bevy::picking::pointer::Location {
            target: bevy::camera::NormalizedRenderTarget::Window(
                bevy::window::WindowRef::Primary
                    .normalize(Some(Entity::PLACEHOLDER))
                    .unwrap(),
            ),
            position: Vec2::ZERO,
        },
        crate::test_pointer::Drag {
            button: PointerButton::Primary,
            distance: Vec2::new(40.0, -60.0),
            delta: Vec2::new(40.0, -60.0),
        },
        handle,
    );
    app.world_mut()
        .resource_mut::<Messages<PointerDrag>>()
        .write(event);
    app.update();
    let next = app.world().resource::<LobbyChatSizing>().resized.unwrap();
    assert!((next.width - start.width - 40.0).abs() < 0.01);
    assert!((next.height - start.height - 60.0).abs() < 0.01);
}
