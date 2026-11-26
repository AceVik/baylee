use super::*;

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<LobbyChatSizing>()
        .init_resource::<LobbyChatPlacing>()
        .insert_resource(ChatUi {
            open: true,
            minimised: false,
            ..default()
        })
        .init_resource::<crate::social::zoom::Zoom>()
        .add_message::<PointerDrag>()
        .add_message::<PointerDragEnd>()
        .add_systems(Update, drag);
    app
}

fn pull(app: &mut App, grip: Entity, distance: Vec2) {
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
            distance,
            delta: distance,
        },
        grip,
    );
    app.world_mut()
        .resource_mut::<Messages<PointerDrag>>()
        .write(event);
    app.update();
}

#[test]
fn the_window_moves_with_its_head_and_docks_again_at_the_foot() {
    let mut app = app();
    let head = app.world_mut().spawn(MoveHandle).id();
    // Up and to the right: the screen's y runs down, the place's bottom up.
    pull(&mut app, head, Vec2::new(200.0, -150.0));
    let moved = app.world().resource::<LobbyChatPlacing>().placed.unwrap();
    assert_eq!(
        moved,
        ChatWindowPlace {
            left: DOCKED.left + 200.0,
            bottom: 150.0
        }
    );
    assert!(!docked(moved));
    // The same drag carried on to nearly where it began docks it.
    pull(&mut app, head, Vec2::new(0.0, -6.0));
    let back = app.world().resource::<LobbyChatPlacing>().placed.unwrap();
    assert_eq!(back.bottom, 0.0);
    assert!(docked(back));
}

#[test]
fn only_the_head_moves_the_window_and_never_while_minimised() {
    let mut app = app();
    let elsewhere = app.world_mut().spawn_empty().id();
    pull(&mut app, elsewhere, Vec2::new(80.0, -80.0));
    assert!(app.world().resource::<LobbyChatPlacing>().placed.is_none());
    let head = app.world_mut().spawn(MoveHandle).id();
    app.world_mut().resource_mut::<ChatUi>().minimised = true;
    pull(&mut app, head, Vec2::new(80.0, -80.0));
    assert!(app.world().resource::<LobbyChatPlacing>().placed.is_none());
}

#[test]
fn a_place_stands_the_whole_window_inside_the_screen() {
    let size = ChatWindowSize {
        width: 400.0,
        height: 300.0,
    };
    let screen = Vec2::new(1280.0, 720.0);
    let far = ChatWindowPlace {
        left: 5000.0,
        bottom: -40.0,
    };
    assert_eq!(
        lobby_panel_place(Some(far), size, screen, 1.0),
        ChatWindowPlace {
            left: 880.0,
            bottom: 0.0
        }
    );
    // At zoom 2 the same screen holds half as many interface pixels.
    let high = ChatWindowPlace {
        left: 100.0,
        bottom: 500.0,
    };
    assert_eq!(
        lobby_panel_place(Some(high), size, screen, 2.0),
        ChatWindowPlace {
            left: 100.0,
            bottom: 60.0
        }
    );
    // Never moved, it stands where it always stood; nothing is not a place.
    assert_eq!(lobby_panel_place(None, size, screen, 1.0), DOCKED);
    let broken = ChatWindowPlace {
        left: f32::NAN,
        bottom: f32::INFINITY,
    };
    assert_eq!(lobby_panel_place(Some(broken), size, screen, 1.0), DOCKED);
    // A window larger than the screen stands at its corner.
    let huge = ChatWindowSize {
        width: 2000.0,
        height: 900.0,
    };
    assert_eq!(
        lobby_panel_place(Some(high), huge, screen, 1.0),
        ChatWindowPlace {
            left: 0.0,
            bottom: 0.0
        }
    );
}
