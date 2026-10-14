use super::{Clicks, UiButton, testing};
use bevy::picking::hover::Hovered;
use bevy::picking::pointer::{PointerAction, PointerButton, PointerId};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct Action(u8);

#[derive(Resource, Default)]
struct Actions(Vec<Action>);

fn app() -> App {
    let mut app = testing::app();
    app.init_resource::<Actions>().add_systems(
        Update,
        |mut clicks: Clicks<Action>, mut actions: ResMut<Actions>| {
            actions.0.extend(clicks.read());
        },
    );
    app
}

fn actions(app: &App) -> &[Action] {
    &app.world().resource::<Actions>().0
}

#[test]
fn a_button_is_released_until_pressed_and_clicks_once_on_release() {
    let mut app = app();
    let button = app.world_mut().spawn((UiButton, Action(1))).id();
    assert!(
        app.world()
            .get::<bevy::ui_widgets::Button>(button)
            .is_some()
    );
    assert!(app.world().get::<Node>(button).is_some());
    assert_eq!(
        app.world()
            .get::<bevy::a11y::AccessibilityNode>(button)
            .expect("button accessibility")
            .role(),
        accesskit::Role::Button
    );
    assert!(matches!(
        app.world().get::<bevy::ui::FocusPolicy>(button),
        Some(bevy::ui::FocusPolicy::Block)
    ));
    assert!(app.world().get::<Pressed>(button).is_none());
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(button),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert!(app.world().get::<Pressed>(button).is_some());
    assert!(app.world().get::<Hovered>(button).expect("hover state").0);
    assert!(actions(&app).is_empty(), "down must not close a panel");
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(button),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert_eq!(actions(&app), [Action(1)]);
    assert_eq!(app.world().resource::<testing::Activations>().0, [button]);
    assert!(app.world().get::<Pressed>(button).is_none());
    testing::frame(&mut app, PointerId::Mouse, Some(button), &[]);
    assert_eq!(actions(&app), [Action(1)], "hover is not another click");
    testing::click(&mut app, button);
    assert_eq!(actions(&app), [Action(1), Action(1)]);
}

#[test]
fn right_and_middle_clicks_cannot_dispatch_domain_actions() {
    let mut app = app();
    let button = app.world_mut().spawn((UiButton, Action(1))).id();
    for mouse_button in [PointerButton::Secondary, PointerButton::Middle] {
        testing::frame(
            &mut app,
            PointerId::Mouse,
            Some(button),
            &[PointerAction::Press(mouse_button)],
        );
        testing::frame(
            &mut app,
            PointerId::Mouse,
            Some(button),
            &[PointerAction::Release(mouse_button)],
        );
        assert!(actions(&app).is_empty());
        assert!(app.world().get::<Pressed>(button).is_none());
    }
    assert_eq!(
        app.world().resource::<testing::Activations>().0.len(),
        2,
        "Bevy emitted Activate; forwarding it would bypass the filter"
    );
}

#[test]
fn a_label_click_stops_at_the_innermost_button_even_for_a_different_action() {
    let mut app = app();
    let outer = app.world_mut().spawn((UiButton, Action(1))).id();
    let inner = app
        .world_mut()
        .spawn((UiButton, Action(2), ChildOf(outer)))
        .id();
    let label = app.world_mut().spawn(ChildOf(inner)).id();
    testing::click(&mut app, label);
    assert_eq!(actions(&app), [Action(2)]);
    assert_eq!(app.world().resource::<testing::Activations>().0, [inner]);
    // Another kind of nested control must not fall through to Action(1).
    app.world_mut().entity_mut(inner).remove::<Action>();
    testing::click(&mut app, label);
    assert_eq!(actions(&app), [Action(2)]);
}

#[test]
fn disabled_controls_do_not_click_and_can_be_enabled_again() {
    let mut app = app();
    let native = app
        .world_mut()
        .spawn((UiButton, Action(1), InteractionDisabled))
        .id();
    let shell = app
        .world_mut()
        .spawn((UiButton, Action(2), crate::shellkit::controls::Disabled))
        .id();
    testing::click(&mut app, native);
    testing::click(&mut app, shell);
    assert!(actions(&app).is_empty());
    assert!(app.world().resource::<testing::Activations>().0.is_empty());
    assert!(app.world().get::<Pressed>(native).is_none());
    assert!(app.world().get::<Pressed>(shell).is_none());
    app.world_mut()
        .entity_mut(native)
        .remove::<InteractionDisabled>();
    app.world_mut()
        .entity_mut(shell)
        .remove::<crate::shellkit::controls::Disabled>();
    testing::click(&mut app, native);
    testing::click(&mut app, shell);
    assert_eq!(actions(&app), [Action(1), Action(2)]);
}

#[test]
fn disabling_while_held_clears_pressed_and_removing_disabled_reenables_the_widget() {
    use crate::shellkit::controls::Disabled;
    let mut app = app();
    let button = app.world_mut().spawn((UiButton, Action(1))).id();
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(button),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert!(app.world().get::<Pressed>(button).is_some());
    app.world_mut().entity_mut(button).insert(Disabled);
    assert!(app.world().get::<InteractionDisabled>(button).is_some());
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(button).is_none());
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(button),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(actions(&app).is_empty());
    assert!(app.world().resource::<testing::Activations>().0.is_empty());
    // A queued native press after the disable must not restore held state.
    app.world_mut().commands().entity(button).insert(Pressed);
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(button).is_none());
    app.world_mut().entity_mut(button).remove::<Disabled>();
    assert!(
        app.world().get::<InteractionDisabled>(button).is_none(),
        "no sticky required marker"
    );
    testing::click(&mut app, button);
    assert_eq!(actions(&app), [Action(1)]);
    assert_eq!(app.world().resource::<testing::Activations>().0, [button]);
}

#[test]
fn reenabling_before_release_does_not_resurrect_a_cancelled_press() {
    use crate::shellkit::controls::Disabled;
    let mut app = app();
    let button = app.world_mut().spawn((UiButton, Action(1))).id();
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(button),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    app.world_mut().entity_mut(button).insert(Disabled);
    app.world_mut().flush();
    app.world_mut().entity_mut(button).remove::<Disabled>();
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(button),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(actions(&app).is_empty());
    assert!(app.world().resource::<testing::Activations>().0.is_empty());
    testing::click(&mut app, button);
    assert_eq!(actions(&app), [Action(1)]);
}

#[test]
fn hovering_without_a_press_does_not_click() {
    let mut app = app();
    let button = app.world_mut().spawn((UiButton, Action(1))).id();
    for target in [Some(button), None, Some(button)] {
        testing::frame(&mut app, PointerId::Mouse, target, &[]);
    }
    assert!(actions(&app).is_empty());
    assert!(app.world().resource::<testing::Activations>().0.is_empty());
}

#[test]
fn two_touch_controls_each_answer_their_own_release() {
    let mut app = app();
    let first = app.world_mut().spawn((UiButton, Action(1))).id();
    let second = app.world_mut().spawn((UiButton, Action(2))).id();
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        Some(first),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    testing::frame(
        &mut app,
        PointerId::Touch(2),
        Some(second),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert!(actions(&app).is_empty());
    testing::frame(
        &mut app,
        PointerId::Touch(2),
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert_eq!(actions(&app), [Action(2)]);
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert_eq!(actions(&app), [Action(2), Action(1)]);
}

#[test]
fn touch_release_can_click_without_mouse_hover() {
    let mut app = app();
    let button = app.world_mut().spawn((UiButton, Action(1))).id();
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        Some(button),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert!(!app.world().get::<Hovered>(button).expect("hover state").0);
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert_eq!(actions(&app), [Action(1)]);
    assert!(app.world().get::<Pressed>(button).is_none());
}

#[test]
fn a_press_and_release_in_one_frame_is_not_lost_or_doubled() {
    let mut app = app();
    let button = app.world_mut().spawn((UiButton, Action(1))).id();
    testing::frame(&mut app, PointerId::Mouse, Some(button), &[]);
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(button),
        &[
            PointerAction::Press(PointerButton::Primary),
            PointerAction::Release(PointerButton::Primary),
        ],
    );
    assert_eq!(actions(&app), [Action(1)]);
    assert!(app.world().get::<Pressed>(button).is_none());
}

#[test]
fn dragging_off_and_releasing_over_another_control_clicks_neither() {
    let mut app = app();
    let first = app.world_mut().spawn((UiButton, Action(1))).id();
    let other = app.world_mut().spawn((UiButton, Action(2))).id();
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(first),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(other),
        &[PointerAction::Move {
            delta: Vec2::splat(20.0),
        }],
    );
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(other),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(actions(&app).is_empty());
    assert!(app.world().get::<Pressed>(first).is_none());
}

#[test]
fn cancelled_touch_does_not_click_on_a_later_release() {
    let mut app = app();
    let button = app.world_mut().spawn((UiButton, Action(1))).id();
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        Some(button),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        Some(button),
        &[PointerAction::Cancel],
    );
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(actions(&app).is_empty());
    assert!(app.world().get::<Pressed>(button).is_none());
}

#[test]
fn a_replaced_control_does_not_inherit_an_unrelated_press() {
    let mut app = app();
    let old = app.world_mut().spawn((UiButton, Action(1))).id();
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(old),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    app.world_mut().entity_mut(old).despawn();
    let replacement = app.world_mut().spawn((UiButton, Action(2))).id();
    testing::frame(&mut app, PointerId::Mouse, Some(replacement), &[]);
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(replacement),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(actions(&app).is_empty());
    testing::click(&mut app, replacement);
    assert_eq!(actions(&app), [Action(2)]);
}
