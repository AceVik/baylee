//! The old release contracts, exercised through Bevy's real pointer
//! observers and the drawer's production handler. Toggles expose a second
//! activation: counting widget events alone would miss a domain double click.

use super::*;
use crate::ui_button::testing;
use bevy::picking::backend::HitData;
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::{
    PointerAction, PointerButton, PointerId, PointerInput, PointerLocation, PointerMap,
};
use bevy::ui::Pressed;

fn app() -> App {
    let mut app = testing::app();
    app.init_resource::<SocialHost>()
        .init_resource::<ChatUi>()
        .add_systems(Update, presses);
    app
}

fn control(app: &mut App, press: DrawerPress) -> Entity {
    let entity = small_button(
        &mut app.world_mut().commands(),
        &UiFonts::default(),
        "test control",
        press,
    );
    app.world_mut().flush();
    entity
}

fn point(app: &mut App, target: Option<Entity>, actions: &[PointerAction]) {
    testing::frame(app, PointerId::Mouse, target, actions);
}

#[test]
fn a_press_released_over_its_control_clicks_once() {
    let mut app = app();
    let toggle = control(&mut app, DrawerPress::Toggle);
    point(
        &mut app,
        Some(toggle),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert!(app.world().get::<Pressed>(toggle).is_some());
    assert!(!app.world().resource::<ChatUi>().drawer, "down is no click");
    point(
        &mut app,
        Some(toggle),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(
        app.world().resource::<ChatUi>().drawer,
        "one toggle, not two"
    );
    assert!(app.world().get::<Pressed>(toggle).is_none());
    point(&mut app, None, &[]);
    point(&mut app, Some(toggle), &[]);
    assert!(
        app.world().resource::<ChatUi>().drawer,
        "hover is no second click"
    );
    assert_eq!(app.world().resource::<testing::Activations>().0, [toggle]);
}

#[test]
fn a_press_released_elsewhere_clicks_nothing() {
    let mut app = app();
    let toggle = control(&mut app, DrawerPress::Toggle);
    point(
        &mut app,
        Some(toggle),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    point(
        &mut app,
        None,
        &[PointerAction::Move {
            delta: Vec2::new(30.0, 0.0),
        }],
    );
    point(
        &mut app,
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(!app.world().resource::<ChatUi>().drawer);
    assert!(app.world().get::<Pressed>(toggle).is_none());
    point(&mut app, Some(toggle), &[]);
    assert!(
        !app.world().resource::<ChatUi>().drawer,
        "returning is not a release"
    );
}

#[test]
fn a_hover_that_was_never_pressed_clicks_nothing() {
    let mut app = app();
    let toggle = control(&mut app, DrawerPress::Toggle);
    for target in [Some(toggle), None, Some(toggle)] {
        point(&mut app, target, &[]);
        assert!(!app.world().resource::<ChatUi>().drawer);
        assert!(app.world().get::<Pressed>(toggle).is_none());
    }
}

#[test]
fn a_release_over_another_control_clicks_neither() {
    let mut app = app();
    let toggle = control(&mut app, DrawerPress::Toggle);
    let more = control(&mut app, DrawerPress::More("alice".into()));
    point(
        &mut app,
        Some(toggle),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    point(
        &mut app,
        Some(more),
        &[PointerAction::Move {
            delta: Vec2::new(30.0, 0.0),
        }],
    );
    point(
        &mut app,
        Some(more),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    let ui = app.world().resource::<ChatUi>();
    assert!(!ui.drawer);
    assert!(ui.drawer_menu.is_none());
    assert!(app.world().get::<Pressed>(toggle).is_none());
    assert!(app.world().get::<Pressed>(more).is_none());
}

#[test]
fn two_controls_pressed_in_one_frame_each_click_on_their_own_release() {
    let mut app = app();
    let toggle = control(&mut app, DrawerPress::Toggle);
    let more = control(&mut app, DrawerPress::More("alice".into()));
    // Queue both raw touch downs before advancing the app once. No widget
    // component or synthetic Click is written by this test.
    for (id, target) in [(PointerId::Touch(1), toggle), (PointerId::Touch(2), more)] {
        app.world_mut().resource_mut::<HoverMap>().insert(
            id,
            [(target, HitData::new(Entity::PLACEHOLDER, 0.0, None, None))].into(),
        );
        let pointer = app.world().resource::<PointerMap>().get_entity(id).unwrap();
        let location = app
            .world()
            .get::<PointerLocation>(pointer)
            .unwrap()
            .location
            .clone()
            .unwrap();
        app.world_mut().write_message(PointerInput::new(
            id,
            location,
            PointerAction::Press(PointerButton::Primary),
        ));
    }
    app.update();
    assert!(app.world().get::<Pressed>(toggle).is_some());
    assert!(app.world().get::<Pressed>(more).is_some());
    assert!(!app.world().resource::<ChatUi>().drawer);
    assert!(app.world().resource::<ChatUi>().drawer_menu.is_none());
    testing::frame(
        &mut app,
        PointerId::Touch(2),
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert_eq!(
        app.world().resource::<ChatUi>().drawer_menu.as_deref(),
        Some("alice")
    );
    assert!(!app.world().resource::<ChatUi>().drawer);
    assert!(app.world().get::<Pressed>(toggle).is_some());
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(app.world().resource::<ChatUi>().drawer);
    assert_eq!(
        app.world().resource::<ChatUi>().drawer_menu.as_deref(),
        Some("alice")
    );
    assert!(app.world().get::<Pressed>(toggle).is_none());
    assert!(app.world().get::<Pressed>(more).is_none());
    assert_eq!(
        app.world().resource::<testing::Activations>().0,
        [more, toggle]
    );
}

#[test]
fn a_second_press_of_the_same_control_clicks_again() {
    let mut app = app();
    let toggle = control(&mut app, DrawerPress::Toggle);
    testing::click(&mut app, toggle);
    assert!(app.world().resource::<ChatUi>().drawer);
    testing::click(&mut app, toggle);
    assert!(!app.world().resource::<ChatUi>().drawer);
    assert_eq!(
        app.world().resource::<testing::Activations>().0,
        [toggle, toggle]
    );
}
