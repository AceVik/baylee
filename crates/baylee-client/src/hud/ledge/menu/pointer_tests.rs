use super::*;
use crate::ui_button::testing;
use bevy::picking::pointer::{PointerAction, PointerButton, PointerId};
use bevy::ui::Pressed;

#[test]
fn the_drawn_hud_burger_toggles_through_the_production_pointer_handler_once() {
    let mut app = crate::input::tests::pointer_app(Duel::default());
    testing::install(&mut app);
    let button = burger(&mut app.world_mut().commands(), &UiFonts::default(), false);
    app.world_mut().flush();
    assert!(
        app.world()
            .get::<bevy::ui_widgets::Button>(button)
            .is_some()
    );
    let label = app.world().get::<Children>(button).unwrap()[0];
    assert_eq!(app.world().get::<Pickable>(label), Some(&Pickable::IGNORE));
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(button),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert!(!app.world().resource::<Duel>().game_menu);
    assert!(app.world().get::<Pressed>(button).is_some());
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(button),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(
        app.world().resource::<Duel>().game_menu,
        "one click opened the game menu"
    );
    assert!(app.world().get::<Pressed>(button).is_none());
    assert_eq!(app.world().resource::<testing::Activations>().0, [button]);
    app.update();
    assert!(app.world().resource::<Duel>().game_menu);
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        Some(button),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert!(app.world().resource::<Duel>().game_menu);
    testing::frame(
        &mut app,
        PointerId::Touch(1),
        None,
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(
        !app.world().resource::<Duel>().game_menu,
        "one touch closed it"
    );
    assert_eq!(
        app.world().resource::<testing::Activations>().0,
        [button, button]
    );
    assert!(app.world().resource::<Duel>().outbox().is_empty());
}

/// The game menu's follow switch (report 30.10.2026): it says the table's
/// follow setting, and a click flips `TableSettings::follow` and says so.
#[test]
fn the_follow_switch_says_its_state_and_a_click_flips_it() {
    use super::switches::{FollowSwitch, priority_switch, show_priority_switch};
    let mut app = crate::input::tests::pointer_app(Duel::default());
    testing::install(&mut app);
    app.insert_resource(crate::settings::ClientSettings::default())
        .add_systems(Update, show_priority_switch);
    let _row = priority_switch(&mut app.world_mut().commands(), &UiFonts::default());
    app.world_mut().flush();
    app.update();
    let label = app
        .world_mut()
        .query_filtered::<Entity, With<FollowSwitch>>()
        .single(app.world())
        .expect("one follow switch");
    let words = |app: &App| app.world().get::<Text>(label).expect("its words").0.clone();
    assert_eq!(words(&app), "Table follows the turn: on");
    let button = app
        .world()
        .get::<ChildOf>(label)
        .expect("in its button")
        .parent();
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(button),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(button),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert!(
        !app.world()
            .resource::<crate::settings::ClientSettings>()
            .table
            .follow,
        "a click turned the follow off"
    );
    app.update();
    assert_eq!(words(&app), "Table follows the turn: off");
}
