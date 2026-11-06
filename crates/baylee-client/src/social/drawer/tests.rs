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

/// A list a person can read through stays whole, whatever it is scrolled to.
#[test]
fn a_short_list_of_friends_draws_every_row() {
    assert_eq!(row_window(DRAWN_WHOLE, 3, 2), 0..DRAWN_WHOLE);
}

/// The window reaches over what is on screen from the top to the last row,
/// and never past the end.
#[test]
fn a_long_list_draws_the_rows_around_where_it_is_scrolled_to() {
    let total = 500;
    let at_top = row_window(total, 0, 3);
    assert_eq!(at_top, 0..WINDOW_ROWS);
    let row = 200;
    let window = row_window(total, row / WINDOW_STEP, 3);
    assert!(window.start < row && row + 30 < window.end, "{window:?}");
    assert_eq!(window.len(), WINDOW_ROWS);
    let at_end = row_window(total, total / WINDOW_STEP, 3);
    assert_eq!(at_end, total - WINDOW_ROWS..total);
}

/// What is not drawn stands in as height, in one piece on each side of the
/// window, so the list's length and scrollbar hold.
#[test]
fn the_rows_left_out_keep_the_lists_length() {
    let mut app = app();
    let mut commands = app.world_mut().commands();
    let mut kids = Vec::new();
    let mut window = RowWindow::new(10..12, 50.0);
    let drawn = (0..30)
        .filter(|_| window.takes_row(&mut commands, &mut kids))
        .count();
    window.close(&mut commands, &mut kids);
    app.update();
    assert_eq!(drawn, 2);
    let heights: Vec<Val> = kids
        .iter()
        .map(|kid| app.world().get::<Node>(*kid).expect("a spacer").height)
        .collect();
    assert_eq!(heights, [Val::Px(500.0), Val::Px(900.0)]);
}

/// The window moves in steps smaller than its reach above the view, so a
/// scroll never lands on rows it has not drawn yet.
#[test]
fn the_window_moves_before_the_view_reaches_its_edge() {
    let pitch = 56.0;
    assert_eq!(window_chunk(0.0, pitch), 0);
    assert_eq!(window_chunk(-30.0, pitch), 0);
    assert_eq!(window_chunk(pitch * 8.5, pitch), 1);
}
