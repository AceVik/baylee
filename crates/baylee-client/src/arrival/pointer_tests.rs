//! Cancellation stays a down action, including a fast down/up pair. The
//! app runs Bevy's input dispatch and the production `cancel` system.

use super::*;
use crate::ui_button::testing;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::picking::pointer::{PointerAction, PointerButton, PointerId};
use bevy::ui::{InteractionDisabled, Pressed};

#[derive(Resource, Default)]
struct Closes(usize);

fn app(destination: Destination) -> (App, Entity) {
    let mut app = testing::app();
    app.insert_resource(Journey {
        destination,
        ..default()
    })
    .init_resource::<Closes>()
    .add_message::<DuelCommand>()
    .add_systems(
        Update,
        (
            cancel,
            |mut messages: MessageReader<DuelCommand>, mut closes: ResMut<Closes>| {
                for message in messages.read() {
                    assert!(matches!(message, DuelCommand::Close));
                    closes.0 += 1;
                }
            },
        )
            .chain(),
    );
    let exit = app.world_mut().spawn((UiButton, Exit)).id();
    (app, exit)
}

fn closes(app: &App) -> usize {
    app.world().resource::<Closes>().0
}

fn frame(app: &mut App, target: Entity, actions: &[PointerAction]) {
    testing::frame(app, PointerId::Mouse, Some(target), actions);
}

#[test]
fn arrival_cancels_on_down_and_never_again_on_release() {
    let (mut app, exit) = app(Destination::Table);
    let label = app.world_mut().spawn((Node::default(), ChildOf(exit))).id();
    frame(
        &mut app,
        label,
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert_eq!(closes(&app), 1, "the down already requested Close");
    assert!(app.world().get::<Pressed>(exit).is_some());
    frame(
        &mut app,
        label,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert_eq!(closes(&app), 1);
    assert!(app.world().get::<Pressed>(exit).is_none());
    assert_eq!(app.world().resource::<testing::Activations>().0, [exit]);
    app.update();
    assert_eq!(
        closes(&app),
        1,
        "Activate was not forwarded as another close"
    );
}

#[test]
fn arrival_catches_a_down_up_pair_even_when_pressed_is_already_removed() {
    let (mut app, exit) = app(Destination::Table);
    // Releases target the previous hover map, like a real pointer that was
    // already over the control before clicking between two rendered frames.
    frame(&mut app, exit, &[]);
    frame(
        &mut app,
        exit,
        &[
            PointerAction::Press(PointerButton::Primary),
            PointerAction::Release(PointerButton::Primary),
        ],
    );
    assert!(app.world().get::<Pressed>(exit).is_none());
    assert_eq!(closes(&app), 1);
}

#[test]
fn arrival_ignores_secondary_and_disabled_but_accepts_touch_down() {
    let (mut app, exit) = app(Destination::Table);
    frame(
        &mut app,
        exit,
        &[
            PointerAction::Press(PointerButton::Secondary),
            PointerAction::Release(PointerButton::Secondary),
        ],
    );
    assert_eq!(closes(&app), 0);
    app.world_mut().entity_mut(exit).insert(InteractionDisabled);
    testing::click(&mut app, exit);
    assert_eq!(closes(&app), 0);
    assert!(app.world().get::<Pressed>(exit).is_none());
    app.world_mut()
        .entity_mut(exit)
        .remove::<InteractionDisabled>();
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        Some(exit),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert_eq!(closes(&app), 1);
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert_eq!(closes(&app), 1);
    assert!(app.world().get::<Pressed>(exit).is_none());
}

#[test]
fn a_nested_button_does_not_press_the_arrival_exit_above_it() {
    let (mut app, exit) = app(Destination::Table);
    let inner = app.world_mut().spawn((UiButton, ChildOf(exit))).id();
    let label = app
        .world_mut()
        .spawn((Node::default(), ChildOf(inner)))
        .id();
    testing::click(&mut app, label);
    assert_eq!(closes(&app), 0);
    assert_eq!(app.world().resource::<testing::Activations>().0, [inner]);
}

#[test]
fn arrival_drains_ignored_presses_before_the_next_destination() {
    for destination in [Destination::Login, Destination::Hidden] {
        let (mut app, exit) = app(destination);
        frame(
            &mut app,
            exit,
            &[PointerAction::Press(PointerButton::Primary)],
        );
        assert_eq!(closes(&app), 0);
        app.world_mut().resource_mut::<Journey>().destination = Destination::Table;
        frame(
            &mut app,
            exit,
            &[PointerAction::Release(PointerButton::Primary)],
        );
        assert_eq!(
            closes(&app),
            0,
            "an earlier down must not cancel the next trip"
        );
        frame(
            &mut app,
            exit,
            &[PointerAction::Press(PointerButton::Primary)],
        );
        assert_eq!(closes(&app), 1);
    }
}

#[test]
fn escape_still_cancels_only_a_table_journey() {
    for destination in [Destination::Login, Destination::Hidden, Destination::Table] {
        let (mut app, _) = app(destination);
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: Key::Escape,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
        assert_eq!(closes(&app), usize::from(destination == Destination::Table));
        app.update();
        assert_eq!(closes(&app), usize::from(destination == Destination::Table));
    }
}
