use super::*;
use crate::ui_button::testing;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::pointer::{PointerAction, PointerButton, PointerId};
use bevy::ui::Pressed;

#[test]
fn a_drawn_replay_button_keeps_its_own_observer_and_asks_once_on_release() {
    let mut app = testing::app();
    app.init_resource::<ReplayAsks>();
    let ask = BarAsk::Host(ReplayAsk::Transport(Command::Step(1)));
    let outer = pill(
        &mut app.world_mut().commands(),
        &UiFonts::default(),
        "picker",
        false,
        BarAsk::TogglePicker,
    );
    let inner = pill(
        &mut app.world_mut().commands(),
        &UiFonts::default(),
        "next",
        false,
        ask,
    );
    app.world_mut().flush();
    app.world_mut().entity_mut(outer).add_child(inner);
    let icon = app
        .world_mut()
        .spawn((Node::default(), ChildOf(inner)))
        .id();
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(icon),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert!(app.world().resource::<ReplayAsks>().0.is_empty());
    assert!(app.world().get::<Pressed>(inner).is_some());
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(icon),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert_eq!(
        app.world().resource::<ReplayAsks>().0,
        [ask],
        "the entity observer received the click, the outer button did not"
    );
    assert_eq!(app.world().resource::<testing::Activations>().0, [inner]);
    assert!(app.world().get::<Pressed>(inner).is_none());
    assert!(app.world().get::<Pressed>(outer).is_none());
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        Some(inner),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert_eq!(app.world().resource::<ReplayAsks>().0, [ask]);
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert_eq!(app.world().resource::<ReplayAsks>().0, [ask, ask]);
    app.update();
    assert_eq!(app.world().resource::<ReplayAsks>().0, [ask, ask]);
}

#[test]
fn a_native_keyboard_activate_is_not_forwarded_to_the_replay_observer() {
    let mut app = testing::app();
    app.init_resource::<ReplayAsks>();
    let button = pill(
        &mut app.world_mut().commands(),
        &UiFonts::default(),
        "picker",
        false,
        BarAsk::TogglePicker,
    );
    app.world_mut().flush();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(button, FocusCause::Navigated);
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Enter,
        logical_key: Key::Enter,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    assert_eq!(app.world().resource::<testing::Activations>().0, [button]);
    assert!(
        app.world().resource::<ReplayAsks>().0.is_empty(),
        "replay keys retain their existing action path"
    );
    assert!(app.world().get::<Pressed>(button).is_none());
}

/// The settings standing over a replay (`lobby::table_settings`) have every
/// key: `Space` there drives nothing. Put away, the same `Space` toggles the
/// replay again.
#[test]
fn the_settings_over_a_replay_keep_its_keys() {
    use crate::replay::{Origin, ReplayStatus};
    use baylee_client_core::replay::{Perspective, Perspectives, Speed, Verify};
    use baylee_core::ids::PlayerId;

    for over in [false, true] {
        let duel = crate::Duel {
            replay: Some(ReplayStatus {
                header: None,
                step: 0,
                steps: 10,
                playing: false,
                speed: Speed::One,
                timeline: None,
                stop: None,
                check: None,
                perspective: Perspective::Seat(PlayerId::new(0)),
                offers: Perspectives {
                    seat: true,
                    public: true,
                    all_hands: true,
                },
                verify: Verify::Stop,
                loading: None,
                problem: None,
                origin: Origin::default(),
                seats: Vec::new(),
            }),
            ..crate::Duel::default()
        };
        let mut lobby = crate::LobbyState::new();
        crate::lobby::table_settings::stand_over_table(&mut lobby);
        if !over {
            crate::lobby::table_settings::close(&mut lobby);
        }
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .insert_state(crate::DuelPhase::Playing)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<BarUi>()
            .init_resource::<ReplayAsks>()
            .insert_resource(duel)
            .insert_resource(lobby)
            .add_systems(Update, replay_keys);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.update();
        let asked = app.world().resource::<ReplayAsks>().0.len();
        if over {
            assert_eq!(asked, 0, "Space drove the replay under the settings");
        } else {
            assert!(asked > 0, "Space drives the replay once they are away");
        }
    }
}
