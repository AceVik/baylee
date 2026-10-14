use super::*;
use crate::ui_button::testing;
use bevy::picking::pointer::{PointerAction, PointerButton, PointerId};
use bevy::ui::{InteractionDisabled, Pressed};

fn app() -> App {
    let mut app = testing::app();
    app.init_resource::<SocialHost>()
        .init_resource::<ChatUi>()
        .add_systems(Update, presses);
    app
}

fn control(app: &mut App, press: ChatPress) -> Entity {
    let entity = text_button(
        &mut app.world_mut().commands(),
        &UiFonts::default(),
        "test control",
        press,
        &LOBBY_INKS,
    );
    app.world_mut().flush();
    entity
}

#[test]
fn nested_window_buttons_only_toggle_the_inner_action_once() {
    let mut app = app();
    let outer = control(&mut app, ChatPress::Friends);
    let inner = control(&mut app, ChatPress::Gear);
    app.world_mut().entity_mut(outer).add_child(inner);
    // A pickable icon inside the button must resolve to the inner control.
    let icon = app
        .world_mut()
        .spawn((Node::default(), ChildOf(inner)))
        .id();
    testing::click(&mut app, icon);
    let ui = app.world().resource::<ChatUi>();
    assert_eq!(ui.filters, Some(ui.tab));
    assert!(!ui.drawer, "the outer button did not also toggle");
    assert_eq!(app.world().resource::<testing::Activations>().0, [inner]);
    assert!(app.world().get::<Pressed>(inner).is_none());
    assert!(app.world().get::<Pressed>(outer).is_none());
    testing::click(&mut app, icon);
    assert!(app.world().resource::<ChatUi>().filters.is_none());
    assert!(!app.world().resource::<ChatUi>().drawer);
}

#[test]
fn a_disabled_window_button_blocks_its_descendants_and_recovers_on_remove() {
    let mut app = app();
    let gear = control(&mut app, ChatPress::Gear);
    let icon = app.world_mut().spawn((Node::default(), ChildOf(gear))).id();
    app.world_mut().entity_mut(gear).insert(InteractionDisabled);
    testing::click(&mut app, icon);
    assert!(app.world().resource::<ChatUi>().filters.is_none());
    assert!(app.world().resource::<testing::Activations>().0.is_empty());
    assert!(app.world().get::<Pressed>(gear).is_none());
    app.world_mut()
        .entity_mut(gear)
        .remove::<InteractionDisabled>();
    testing::click(&mut app, icon);
    let ui = app.world().resource::<ChatUi>();
    assert_eq!(ui.filters, Some(ui.tab));
    assert_eq!(app.world().resource::<testing::Activations>().0, [gear]);
}

#[test]
fn secondary_activate_is_ignored_but_touch_toggles_once_on_release() {
    let mut app = app();
    let gear = control(&mut app, ChatPress::Gear);
    for action in [
        PointerAction::Press(PointerButton::Secondary),
        PointerAction::Release(PointerButton::Secondary),
    ] {
        testing::frame(&mut app, PointerId::Mouse, Some(gear), &[action]);
        assert!(app.world().resource::<ChatUi>().filters.is_none());
    }
    assert_eq!(app.world().resource::<testing::Activations>().0, [gear]);
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        Some(gear),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert!(app.world().resource::<ChatUi>().filters.is_none());
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    let ui = app.world().resource::<ChatUi>();
    assert_eq!(ui.filters, Some(ui.tab));
    app.update();
    assert!(
        app.world().resource::<ChatUi>().filters.is_some(),
        "no delayed second toggle"
    );
    assert!(app.world().get::<Pressed>(gear).is_none());
    assert_eq!(
        app.world().resource::<testing::Activations>().0,
        [gear, gear]
    );
}
