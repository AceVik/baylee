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
        .init_resource::<CameraPose>()
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

/// Visits that seat the way the `F` key and a press on its button do.
fn look_at_a_seat(app: &mut App) {
    let mut duel = app.world_mut().resource_mut::<Duel>();
    crate::input::navigate_to_player(&mut duel, opponent());
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
        app.world().resource::<Duel>().visiting.is_none(),
        "and it sends the camera visiting on the way"
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

/// And the other direction: a visit is a view the player asked for, so it
/// stays a visit through a resize — re-fitted to the new window, never
/// dropped back to the home shot (DESIGN-v7 §2.4).
///
/// This is the counter-test the test above needs. Without it a rig that
/// nothing could move for a quite different reason — the system unhooked,
/// the layout missing — would pass by standing still.
#[test]
fn looking_at_one_seat_holds_the_visit() {
    let mut app = app(WINDOW);
    let before = *app.world().resource::<CameraRig>();

    look_at_a_seat(&mut app);
    let after = *app.world().resource::<CameraRig>();
    assert_ne!(after.target, before.target, "the seat was never framed");
    assert_eq!(
        app.world().resource::<Duel>().visiting,
        Some(opponent()),
        "a seat the player asked to see is the visit"
    );

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
    let visit = CameraRig::visit(&layout, Canvas::hud(wider), opponent(), Shot::default())
        .expect("the seat is at the table")
        .0;
    assert_eq!(
        *app.world().resource::<CameraRig>(),
        visit,
        "a resize re-fits the visit rather than taking it away"
    );
}

/// `navigate_home` is the way back, and it is the *only* way back a key
/// or a chip needs to know about: it clears the visit, and the next tick
/// frames the whole table again.
#[test]
fn going_home_gives_the_camera_back_to_the_table() {
    let mut app = app(WINDOW);
    look_at_a_seat(&mut app);
    assert!(app.world().resource::<Duel>().visiting.is_some());

    let mut duel = app.world_mut().resource_mut::<Duel>();
    crate::input::navigate_home(&mut duel);
    app.update();

    let layout = app
        .world()
        .resource::<Duel>()
        .layout
        .clone()
        .expect("a layout");
    assert!(app.world().resource::<Duel>().visiting.is_none());
    assert_eq!(
        *app.world().resource::<CameraRig>(),
        CameraRig::home(&layout, Canvas::hud(WINDOW)),
        "and the table framed itself again on the next tick"
    );
}

/// A visit moves no card: the layout every `Motion` target is read from is
/// the same before and after it, and after the return (DESIGN-v7 §2.5).
#[test]
fn a_visit_moves_no_card() {
    let mut app = app(WINDOW);
    let before = app.world().resource::<Duel>().layout.clone();
    look_at_a_seat(&mut app);
    assert_eq!(
        app.world().resource::<Duel>().layout,
        before,
        "a visit re-seated the table"
    );
    let mut duel = app.world_mut().resource_mut::<Duel>();
    crate::input::navigate_home(&mut duel);
    app.update();
    assert_eq!(app.world().resource::<Duel>().layout, before);
}

fn view(active: u8) -> baylee_view::PlayerView {
    let mut view = baylee_client_core::test_support::ViewBuilder::new(4).build();
    view.active = PlayerId::new(active);
    view
}

/// The camera comes home by itself on my turn's start and on my own combat
/// question, and on nothing else: not another seat's turn, not my priority.
#[test]
fn the_camera_comes_home_on_my_turn_and_my_combat_question_only() {
    let mut duel = Duel::default();
    duel.receive_view(view(1));
    duel.visiting = Some(PlayerId::new(2));
    duel.receive_view(view(2));
    assert_eq!(duel.visiting, Some(PlayerId::new(2)), "no auto-follow");
    duel.receive_choice(baylee_engine::choice::Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::default(),
    });
    assert_eq!(
        duel.visiting,
        Some(PlayerId::new(2)),
        "my priority keeps the visit"
    );
    duel.receive_choice(baylee_engine::choice::Pending::ChooseBlockers {
        demands: Vec::new(),
        player: PlayerId::new(0),
        attacker: PlayerId::new(2),
        blockers: Vec::new(),
        capacity: Vec::new(),
        obeying: Vec::new(),
        bounds: Vec::new(),
    });
    assert_eq!(duel.visiting, None, "my blockers bring it home");
    duel.visiting = Some(PlayerId::new(3));
    duel.receive_view(view(0));
    assert_eq!(duel.visiting, None, "my turn brings it home");
}

/// A visit is a timed orbit: the shown camera reaches the rig in about the
/// orbit's 0.55 s and not in the first frame; under reduced motion it is
/// there the same frame.
#[test]
fn a_visit_orbits_in_its_time_and_cuts_under_reduced_motion() {
    for still in [false, true] {
        let mut app = app(WINDOW);
        app.init_resource::<ShownRig>()
            .init_resource::<Time>()
            .init_resource::<crate::prefs::Prefs>()
            .add_systems(Update, apply_camera_rig.after(frame_table));
        if still {
            let mut prefs = app.world_mut().resource_mut::<crate::prefs::Prefs>();
            prefs.edit().reduce_motion = true;
        }
        app.update();
        look_at_a_seat(&mut app);
        let step = std::time::Duration::from_secs_f32(1.0 / 60.0);
        let mut frames = 0;
        loop {
            let shown = app.world().resource::<ShownRig>().rig().expect("shown");
            let target = *app.world().resource::<CameraRig>();
            if shown == target {
                break;
            }
            frames += 1;
            assert!(frames < 60, "the orbit never arrived");
            app.world_mut().resource_mut::<Time>().advance_by(step);
            app.update();
        }
        if still {
            assert_eq!(frames, 0, "reduced motion cuts");
        } else {
            assert!(
                (20..=34).contains(&frames),
                "the orbit took {frames} frames at 60 Hz, not about 0.55 s"
            );
        }
    }
}
