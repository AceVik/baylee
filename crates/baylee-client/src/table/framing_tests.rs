use super::*;
use baylee_core::ids::PlayerId;
use bevy::input::gestures::{PanGesture, PinchGesture};
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::picking::events::Scroll;

/// A laptop's window, in logical pixels.
const WINDOW: Vec2 = Vec2::new(1728.0, 1052.0);

fn app(window: Vec2) -> App {
    let mut app = App::new();
    app.add_message::<MouseMotion>()
        .add_message::<MouseWheel>()
        .add_message::<Pointer<Scroll>>()
        .add_message::<PanGesture>()
        .add_message::<PinchGesture>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<CameraRig>()
        .init_resource::<Duel>()
        .add_systems(Update, frame_table);
    let mut w = Window::default();
    w.resolution.set(window.x, window.y);
    app.world_mut().spawn(w);
    let seats: Vec<PlayerId> = (0..2).map(PlayerId::new).collect();
    let layout = TableLayout::new(&seats, Canvas::hud(window).aspect(), None);
    app.world_mut().resource_mut::<Duel>().layout = Some(layout);
    app.update();
    app
}

/// Every gesture that ever moved this table, in one frame.
///
/// The arrows with shift held are the owner's own report — *„Aktuell wenn
/// ich die Pfeiltasten betätige sammt Shift, dann bedient es nicht das
/// Textfeld, sondern die Kamera vom Tisch"* — and the three buttons, the
/// wheel, the two-finger pan and the pinch are the rest of the set as it
/// stood over the three removals: orbit, then zoom, then this.
fn every_gesture(app: &mut App) {
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        for key in [
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
            KeyCode::ShiftLeft,
        ] {
            keys.press(key);
        }
    }
    {
        let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        for button in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
            buttons.press(button);
        }
    }
    app.world_mut()
        .resource_mut::<Messages<MouseMotion>>()
        .write(MouseMotion {
            delta: Vec2::new(40.0, 20.0),
        });
    app.world_mut()
        .resource_mut::<Messages<MouseWheel>>()
        .write(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -4.0,
            window: Entity::PLACEHOLDER,
            phase: bevy::input::touch::TouchPhase::Moved,
        });
    app.world_mut()
        .resource_mut::<Messages<PanGesture>>()
        .write(PanGesture(Vec2::new(30.0, 30.0)));
    app.world_mut()
        .resource_mut::<Messages<PinchGesture>>()
        .write(PinchGesture(0.4));
    app.update();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.clear();
    }
    let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
    buttons.clear();
}

/// The one seat that is not the local one, which is what there is to look
/// at: the harness seats two.
fn opponent() -> PlayerId {
    PlayerId::new(1)
}

/// Frames that seat the way the `F` key and a tap on its board do.
fn look_at_a_seat(app: &mut App) {
    let mut duel = app.world_mut().remove_resource::<Duel>().expect("a duel");
    let mut rig = app
        .world_mut()
        .remove_resource::<CameraRig>()
        .expect("a rig");
    crate::input::navigate_to_player(&mut duel, &mut rig, opponent());
    app.world_mut().insert_resource(duel);
    app.world_mut().insert_resource(rig);
    app.update();
}

/// Three owner reports, one assertion: *„kaum berühre ich mit der Maus
/// was, drehe ich den Tisch etwas"*, *„Das Zoom in/out sollte eh weg!"*,
/// and now the whole of it — keyboard included.
#[test]
fn nothing_a_hand_does_moves_the_table() {
    let mut app = app(WINDOW);
    let before = *app.world().resource::<CameraRig>();
    every_gesture(&mut app);
    assert_eq!(
        *app.world().resource::<CameraRig>(),
        before,
        "a hand still moves the table"
    );
    assert!(
        !app.world().resource::<Duel>().camera_held,
        "and it takes the camera off the table on the way"
    );
}

/// The half of the first report nobody could see: the framing used to stop
/// following on the first pixel, so the table never came back into frame
/// again — not on a resize, not ever.
#[test]
fn the_table_is_still_framed_after_every_gesture() {
    let mut app = app(WINDOW);
    every_gesture(&mut app);

    let wider = Vec2::new(2400.0, 1052.0);
    let mut windows = app.world_mut().query::<&mut Window>();
    windows
        .iter_mut(app.world_mut())
        .next()
        .expect("a window")
        .resolution
        .set(wider.x, wider.y);
    app.update();

    let layout = app
        .world()
        .resource::<Duel>()
        .layout
        .clone()
        .expect("a layout");
    assert_eq!(
        *app.world().resource::<CameraRig>(),
        CameraRig::home(&layout, Canvas::hud(wider)),
        "a wider window re-frames the table"
    );
}

/// And the other direction, which is the whole of what is left: looking at
/// one seat *is* a view the player asked for, so it holds — and holding is
/// what stops the next resize from taking it away again.
///
/// This is the counter-test the test above needs. Without it a rig that
/// nothing could move for a quite different reason — the system unhooked,
/// the layout missing — would pass by standing still.
#[test]
fn looking_at_one_seat_holds_the_camera() {
    let mut app = app(WINDOW);
    let before = *app.world().resource::<CameraRig>();

    look_at_a_seat(&mut app);
    let after = *app.world().resource::<CameraRig>();
    assert_ne!(after.target, before.target, "the seat was never framed");
    assert!(
        app.world().resource::<Duel>().camera_held,
        "a seat the player asked to see can only mean the camera"
    );

    let mut windows = app.world_mut().query::<&mut Window>();
    windows
        .iter_mut(app.world_mut())
        .next()
        .expect("a window")
        .resolution
        .set(2400.0, 1052.0);
    app.update();
    assert_eq!(
        *app.world().resource::<CameraRig>(),
        after,
        "a resize does not take back a view the player asked for"
    );
}

/// `navigate_home` is the way back, and it is the *only* way back a key
/// or a chip needs to know about: it asks for the one rig that means
/// nobody aimed this.
#[test]
fn going_home_gives_the_camera_back_to_the_table() {
    let mut app = app(WINDOW);
    look_at_a_seat(&mut app);
    assert!(app.world().resource::<Duel>().camera_held);

    let mut duel = app.world_mut().remove_resource::<Duel>().expect("a duel");
    let mut rig = app
        .world_mut()
        .remove_resource::<CameraRig>()
        .expect("a rig");
    crate::input::navigate_home(&mut duel, &mut rig);
    app.world_mut().insert_resource(duel);
    app.world_mut().insert_resource(rig);
    app.update();

    let layout = app
        .world()
        .resource::<Duel>()
        .layout
        .clone()
        .expect("a layout");
    assert!(!app.world().resource::<Duel>().camera_held);
    assert_eq!(
        *app.world().resource::<CameraRig>(),
        CameraRig::home(&layout, Canvas::hud(WINDOW)),
        "and the table framed itself again on the next tick"
    );
}
