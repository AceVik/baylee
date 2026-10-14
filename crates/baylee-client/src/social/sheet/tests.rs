use super::*;
use crate::ui_button::testing;
use baylee_client_core::social::{ChatPrefs, Me};
use bevy::picking::pointer::{PointerAction, PointerButton, PointerId};

#[test]
fn a_sheet_click_changes_the_machine_preference_once_and_only_on_release() {
    // No host session or store: act updates the real Social machine, while
    // carry_out cannot perform network, clipboard or filesystem effects.
    let (machine, _) = Social::start(
        Me {
            tag: "0001".into(),
            handle: "Guest#0001".into(),
            guest: true,
            gateway: String::new(),
        },
        None,
        None,
        ChatPrefs::default(),
        1_000,
        &mut crate::social::OsEntropy,
    );
    let original = machine.prefs().timestamps;
    let mut app = testing::app();
    app.insert_resource(SocialHost {
        machine: Some(machine),
        ..default()
    })
    .init_resource::<ChatUi>()
    .add_systems(Update, presses);
    let target = button(
        &mut app.world_mut().commands(),
        &UiFonts::default(),
        "timestamps",
        SheetPress::Timestamps,
        original,
    );
    app.world_mut().flush();
    let timestamps = |app: &App| {
        app.world()
            .resource::<SocialHost>()
            .machine()
            .unwrap()
            .prefs()
            .timestamps
    };
    for action in [
        PointerAction::Press(PointerButton::Secondary),
        PointerAction::Release(PointerButton::Secondary),
    ] {
        testing::frame(&mut app, PointerId::Mouse, Some(target), &[action]);
        assert_eq!(timestamps(&app), original);
    }
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(target),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert_eq!(timestamps(&app), original);
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(target),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    assert_eq!(
        timestamps(&app),
        !original,
        "one change to the actual preference"
    );
    app.update();
    assert_eq!(timestamps(&app), !original);
    testing::click(&mut app, target);
    assert_eq!(timestamps(&app), original, "a fresh click toggles it back");
    assert_eq!(
        app.world().resource::<testing::Activations>().0,
        [target, target, target]
    );
}
