//! The front door by keys alone (WP1; `KEYBOARD.md` §1.3, §7.2, §8 W1/W2,
//! §9.5, §9.6): the faces' Tab walks read from their `TabOrder` tables,
//! the bridge between the kit's focus and the lobby's caret, Enter on a
//! focused control, Esc going back a face.

#[allow(clippy::wildcard_imports)]
use super::*;

use crate::shellkit::focus::{FocusReport, Stop};
use baylee_client_core::lobby::Face;
use bevy::input::ButtonState;
use bevy::input_focus::InputFocus;

/// One key as a keyboard sends it, then two frames.
pub(super) fn press_key(app: &mut App, code: KeyCode, logical: Key, held: &[KeyCode]) {
    {
        let mut codes = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        codes.reset_all();
        for k in held {
            codes.press(*k);
        }
        codes.press(code);
    }
    let text = match &logical {
        Key::Character(t) => Some(t.clone()),
        Key::Space => Some(" ".into()),
        _ => None,
    };
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: code,
            logical_key: logical.clone(),
            state,
            text: text.clone(),
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.update();
    app.update();
}

pub(super) fn tab(app: &mut App, back: bool) {
    let held: &[KeyCode] = if back { &[KeyCode::ShiftLeft] } else { &[] };
    press_key(app, KeyCode::Tab, Key::Tab, held);
}

pub(super) fn focused(app: &App) -> Option<Stop> {
    app.world().resource::<FocusReport>().stop
}

pub(super) fn focused_id(app: &App) -> Option<&'static str> {
    focused(app).map(|s| s.id)
}

/// The stops drawn for `table`, in no order.
pub(super) fn drawn(app: &mut App, table: &str) -> Vec<&'static str> {
    let mut ids: Vec<&'static str> = app
        .world_mut()
        .query::<&Stop>()
        .iter(app.world())
        .filter(|s| s.table == table)
        .map(|s| s.id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// A front door at a closed beta that takes guests: the sign-in face.
fn at_the_door() -> App {
    let mut app = headless();
    // As the client runs: `bevy_input_focus`'s dispatch clears the focus of
    // an entity a rebuild despawned before the next key is read.
    app.add_systems(
        PreUpdate,
        |mut focus: ResMut<InputFocus>, alive: Query<()>| {
            if focus.get().is_some_and(|e| alive.get(e).is_err()) {
                focus.clear();
            }
        },
    );
    {
        let mut state = app.world_mut().resource_mut::<LobbyState>();
        state.lobby.set_registration(Registration::Invite);
        state.lobby.set_guests_enabled(true);
        state.lobby.focus_on(Field::Username);
    }
    settle(&mut app);
    app.update();
    app
}

/// §9.5: the sign-in face draws only stops its table names, and Tab from
/// the username visits them in the table's order, wrapping; Shift+Tab walks
/// back.
#[test]
fn tab_walks_the_sign_in_face_in_its_tables_order() {
    let mut app = at_the_door();
    let table = &super::super::front::keys::SIGN_IN;
    let drawn = drawn(&mut app, table.name);
    for id in &drawn {
        assert!(
            table.stops.contains(id),
            "{id} is drawn but not in the table"
        );
    }
    let order: Vec<&str> = table
        .stops
        .iter()
        .copied()
        .filter(|id| drawn.contains(id))
        .collect();
    assert!(order.len() >= 10, "{order:?}");
    assert_eq!(
        focused_id(&app),
        Some("username"),
        "focus starts in the username"
    );
    let start = order.iter().position(|id| *id == "username").unwrap();
    for step in 1..=order.len() {
        tab(&mut app, false);
        let want = order[(start + step) % order.len()];
        assert_eq!(focused_id(&app), Some(want), "Tab #{step}");
    }
    tab(&mut app, true);
    assert_eq!(
        focused_id(&app),
        Some(order[(start + order.len() - 1) % order.len()]),
        "Shift+Tab"
    );
}

/// The bridge: focus on a control parks the caret (a letter goes nowhere);
/// focus walked back onto a field takes the caret there.
#[test]
fn a_letter_typed_on_a_button_goes_nowhere() {
    let mut app = at_the_door();
    // Username → Password → eye → Sign in.
    for _ in 0..3 {
        tab(&mut app, false);
    }
    assert_eq!(focused_id(&app), Some("submit"));
    press_key(&mut app, KeyCode::KeyX, Key::Character("x".into()), &[]);
    {
        let state = app.world().resource::<LobbyState>();
        assert!(state.lobby.caret_parked());
        assert_eq!(state.lobby.field(Field::Username), "");
        assert_eq!(state.lobby.field(Field::Password), "");
    }
    tab(&mut app, true);
    tab(&mut app, true);
    assert_eq!(focused_id(&app), Some("password"));
    press_key(&mut app, KeyCode::KeyX, Key::Character("x".into()), &[]);
    let state = app.world().resource::<LobbyState>();
    assert!(!state.lobby.caret_parked());
    assert_eq!(state.lobby.field(Field::Password), "x");
}

/// W1 step 2: Tab ×5 from the username reaches Play as guest; Enter opens
/// the guest's face with the caret in its name; Esc goes back.
#[test]
fn enter_on_play_as_guest_opens_the_guests_face() {
    let mut app = at_the_door();
    for _ in 0..5 {
        tab(&mut app, false);
    }
    assert_eq!(focused_id(&app), Some("guest"));
    press_key(&mut app, KeyCode::Enter, Key::Enter, &[]);
    {
        let state = app.world().resource::<LobbyState>();
        assert_eq!(state.lobby.face(), Face::Guest);
        assert_eq!(state.lobby.focus(), Field::GuestName);
    }
    settle(&mut app);
    app.update();
    assert_eq!(
        focused(&app).map(|s| (s.table, s.id)),
        Some(("front-guest", "guest-name"))
    );
    press_key(
        &mut app,
        KeyCode::KeyA,
        Key::Character("A".into()),
        &[KeyCode::ShiftLeft],
    );
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::GuestName),
        "A"
    );
    press_key(&mut app, KeyCode::Escape, Key::Escape, &[]);
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.face(),
        Face::SignIn
    );
}

/// Esc on the sign-in face goes back to the gateways, whatever has focus.
#[test]
fn escape_from_a_button_on_the_sign_in_face_goes_back_to_the_gateways() {
    let mut app = at_the_door();
    for _ in 0..3 {
        tab(&mut app, false);
    }
    assert_eq!(focused_id(&app), Some("submit"));
    press_key(&mut app, KeyCode::Escape, Key::Escape, &[]);
    assert!(!app.world().resource::<LobbyState>().lobby.gateway_chosen());
}

/// The kit's focus is always on something on the front door's face: after
/// every rebuild a key brought, it is still a stop, never a lost entity.
#[test]
fn focus_survives_the_rebuild_a_keystroke_brings() {
    let mut app = at_the_door();
    for ch in ["a", "d", "a"] {
        press_key(&mut app, KeyCode::KeyA, Key::Character(ch.into()), &[]);
        let entity = app.world().resource::<InputFocus>().get();
        assert!(entity.is_some_and(|e| app.world().get::<Stop>(e).is_some()));
    }
    assert_eq!(
        app.world()
            .resource::<LobbyState>()
            .lobby
            .field(Field::Username),
        "ada"
    );
}

/// Enter on a focused front-door control runs its press once: the front
/// door's activation and the lobby screens' (`focusing::keys_press`) each
/// answer their own tables, so a toggle is not toggled back the same frame.
#[test]
fn enter_on_a_front_door_toggle_turns_it_once() {
    let mut app = headless();
    app.insert_resource(crate::settings::ClientSettings::default());
    app.update();
    let music = {
        let mut query = app.world_mut().query::<(Entity, &Press, &Stop)>();
        query
            .iter(app.world())
            .find(|(_, p, _)| **p == Press::Shared(SharedPress::ToggleMusic))
            .map(|(e, _, _)| e)
            .expect("the text row's Music")
    };
    let heard = |app: &App| {
        app.world()
            .resource::<crate::settings::ClientSettings>()
            .music
            .muted()
    };
    let before = heard(&app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(music, bevy::input_focus::FocusCause::Navigated);
    press_key(&mut app, KeyCode::Enter, Key::Enter, &[]);
    assert_ne!(heard(&app), before, "Enter turned the music once");
}

/// The primary button going down on a control: what a mouse sends before
/// bevy decides whether it was a `Click` (only on the entity pressed).
fn button_down(app: &mut App, entity: Entity) {
    app.world_mut().write_message(aimed(
        entity,
        bevy::picking::events::Press {
            button: PointerButton::Primary,
            hit: bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
            count: 1,
        },
    ));
}

/// The primary button coming up with the pointer over `over`, as bevy
/// delivers it: the pointer's own release ([`PointerInput`]), this frame's
/// hover over `over`, and the `Release` sent to `last`, the entity hovered
/// the frame *before* (bevy's rule), which is the pressed button itself on a
/// release the very next frame.
///
/// [`PointerInput`]: bevy::picking::pointer::PointerInput
fn button_up_after(app: &mut App, last: Entity, over: Entity) {
    use bevy::picking::hover::HoverMap;
    use bevy::picking::pointer::{PointerAction, PointerId, PointerInput};
    let hit = bevy::picking::backend::HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
    let mut hover = HoverMap::default();
    hover
        .entry(PointerId::Mouse)
        .or_default()
        .insert(over, hit.clone());
    app.world_mut().insert_resource(hover);
    let location = aimed(over, ()).pointer_location;
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location,
        PointerAction::Release(PointerButton::Primary),
    ));
    app.world_mut().write_message(aimed(
        last,
        bevy::picking::events::Release {
            button: PointerButton::Primary,
            hit,
        },
    ));
}

/// The primary button coming up on whatever the pointer is over by then,
/// held long enough for the hover to have caught up with it.
fn button_up(app: &mut App, entity: Entity) {
    button_up_after(app, entity, entity);
}

/// The owner's "two clicks" (09.10.2026): with the username holding the
/// caret, the press on Create account moves focus onto the button, which
/// parks the caret, which rebuilds the face before the button comes up, so
/// bevy has no pressed entity to send a `Click` to. One click is one press:
/// the release on the button drawn again in its place is the click.
#[test]
fn one_click_on_a_button_while_a_field_has_the_caret_is_enough() {
    let mut app = at_the_door();
    assert_eq!(
        focused_id(&app),
        Some("username"),
        "the form focused a field"
    );
    let create = Press::Front(FrontPress::ToggleRegistering);
    let pressed = press_target(&mut app, create);
    button_down(&mut app, pressed);
    app.update();
    app.update();
    assert!(
        app.world().get_entity(pressed).is_err(),
        "the press rebuilt the face (the case this is about)"
    );
    let again = press_target(&mut app, create);
    button_up(&mut app, again);
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.face(),
        Face::Create,
        "one click opened Create account"
    );
}

/// The fast click (the Windows session, 09.10.2026): the press rebuilt the
/// face and the button came up the very next frame, before the hover caught
/// up, so bevy's `Release` went to the despawned button and the click was
/// lost; dev-control's press, a touchpad tap and a quick mouse all did that.
/// It is one click, answered once.
#[test]
fn a_fast_click_while_a_field_has_the_caret_is_answered_once() {
    let mut app = at_the_door();
    assert_eq!(focused_id(&app), Some("username"));
    let create = Press::Front(FrontPress::ToggleRegistering);
    let pressed = press_target(&mut app, create);
    button_down(&mut app, pressed);
    app.update();
    app.update();
    assert!(
        app.world().get_entity(pressed).is_err(),
        "the press rebuilt the face (the case this is about)"
    );
    let again = press_target(&mut app, create);
    button_up_after(&mut app, pressed, again);
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.face(),
        Face::Create,
        "one fast click opened Create account"
    );
}

/// Press and release in one frame, before anything is rebuilt: bevy sends
/// its `Click` to the button still standing, and that is the one answer —
/// the release does not count a second time (twice would toggle back).
#[test]
fn a_press_and_release_in_one_frame_is_answered_once() {
    let mut app = at_the_door();
    let create = Press::Front(FrontPress::ToggleRegistering);
    let pressed = press_target(&mut app, create);
    button_down(&mut app, pressed);
    button_up(&mut app, pressed);
    tap(&mut app, pressed);
    app.update();
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.face(),
        Face::Create,
        "pressed once, answered once"
    );
}

/// A click bevy does send (nothing was rebuilt) is answered once, not once
/// for the click and again for the release.
#[test]
fn a_click_that_survived_is_answered_once() {
    let mut app = at_the_door();
    let create = Press::Front(FrontPress::ToggleRegistering);
    // A first press puts focus on the button, so the second rebuilds nothing.
    let target = press_target(&mut app, create);
    button_down(&mut app, target);
    app.update();
    app.update();
    let target = press_target(&mut app, create);
    button_down(&mut app, target);
    app.update();
    button_up(&mut app, target);
    tap(&mut app, target);
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<LobbyState>().lobby.face(),
        Face::Create,
        "pressed once, not twice (twice would toggle back)"
    );
}
