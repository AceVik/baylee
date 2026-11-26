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

#[test]
fn sheet_uses_single_layer_glass_and_preserves_scroll_across_rebuilds() {
    let (machine, _) = Social::start(
        Me {
            tag: "AB12".into(),
            handle: "Alice#AB12".into(),
            guest: false,
            gateway: "https://example.test".into(),
        },
        None,
        None,
        ChatPrefs::default(),
        1_000,
        &mut crate::social::OsEntropy,
    );
    let mut app = testing::app();
    app.insert_resource(UiFonts::default())
        .init_resource::<crate::social::zoom::Zoom>()
        .insert_resource(SocialHost {
            machine: Some(machine),
            ..default()
        })
        .insert_resource(ChatUi {
            sheet: Some(Sheet::Privacy),
            ..default()
        })
        .add_systems(Update, sync);

    app.update();
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<SheetRoot>>()
        .single(app.world())
        .unwrap();
    let card = app.world().get::<Children>(root).unwrap()[0];
    assert_eq!(
        app.world().get::<BackgroundColor>(card).unwrap().0,
        Color::NONE,
        "Sheet card must not double-composite BackgroundColor under its glass"
    );
    assert!(
        app.world()
            .get::<bevy::ui::BackgroundGradient>(card)
            .is_some()
    );

    let list = app
        .world_mut()
        .query_filtered::<Entity, With<SheetList>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .get_mut::<bevy::ui::ScrollPosition>(list)
        .unwrap()
        .y = 144.0;

    // Triggering a rebuild on the same sheet preserves ScrollPosition.
    app.world_mut().resource_mut::<ChatUi>().forget_armed = true;
    app.update();
    let rebuilt_list = app
        .world_mut()
        .query_filtered::<Entity, With<SheetList>>()
        .single(app.world())
        .unwrap();
    assert_ne!(rebuilt_list, list);
    assert!(
        (app.world()
            .get::<bevy::ui::ScrollPosition>(rebuilt_list)
            .unwrap()
            .y
            - 144.0)
            .abs()
            < f32::EPSILON
    );

    // Switching to a different sheet starts cleanly at the top.
    app.world_mut().resource_mut::<ChatUi>().sheet = Some(Sheet::Encryption(None));
    app.update();
    let other_list = app
        .world_mut()
        .query_filtered::<Entity, With<SheetList>>()
        .single(app.world())
        .unwrap();
    assert!(
        app.world()
            .get::<bevy::ui::ScrollPosition>(other_list)
            .unwrap()
            .y
            .abs()
            < f32::EPSILON
    );
}

#[test]
fn friends_drawer_alone_never_blocks_behind_encryption_sheet_and_close_dismisses_first_run() {
    let (machine, _) = Social::start(
        Me {
            tag: "AB12".into(),
            handle: "Alice#AB12".into(),
            guest: false,
            gateway: "https://example.test".into(),
        },
        None,
        None,
        ChatPrefs::default(),
        1_000,
        &mut crate::social::OsEntropy,
    );
    assert!(!machine.encryption_note().seen);
    let mut app = testing::app();
    app.insert_resource(UiFonts::default())
        .init_resource::<crate::social::zoom::Zoom>()
        .insert_resource(SocialHost {
            machine: Some(machine),
            ..default()
        })
        .insert_resource(ChatUi {
            drawer: true,
            ..default()
        });
    install(&mut app);

    app.update();
    assert!(
        app.world().resource::<ChatUi>().sheet.is_none(),
        "opening Friends List alone must not cover it with the encryption modal"
    );

    // Opening Chat triggers the first-run encryption notice once.
    app.world_mut().resource_mut::<ChatUi>().open = true;
    app.update();
    assert_eq!(
        app.world().resource::<ChatUi>().sheet,
        Some(Sheet::Encryption(None))
    );

    // Pressing Escape also dismisses the first-run encryption notice and marks it seen.
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world())
        .expect("primary window");
    app.world_mut()
        .resource_mut::<Messages<bevy::input::keyboard::KeyboardInput>>()
        .write(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
    app.update();
    assert!(
        app.world().resource::<ChatUi>().sheet.is_none(),
        "Escape must dismiss the first-run encryption notice without re-opening next frame"
    );
    assert!(
        app.world()
            .resource::<SocialHost>()
            .machine()
            .unwrap()
            .encryption_note()
            .seen
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();

    // Re-open Privacy sheet and verify clicking Close (or backdrop) dismisses it too.
    app.world_mut().resource_mut::<ChatUi>().sheet = Some(Sheet::Privacy);
    app.update();
    let close = app
        .world_mut()
        .query::<(Entity, &SheetPress)>()
        .iter(app.world())
        .find_map(|(e, p)| (*p == SheetPress::Close).then_some(e))
        .expect("Close button present on sheet");
    testing::click(&mut app, close);
    app.update();
    assert!(app.world().resource::<ChatUi>().sheet.is_none());
}
