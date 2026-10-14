use super::*;
use crate::ui_button::testing;
use bevy::picking::pointer::{PointerAction, PointerId};

#[test]
fn row_actions_ignore_right_click_and_wait_for_primary_release() {
    let mut app = testing::app();
    let key = baylee_client_core::social::LineKey {
        conv: baylee_client_core::social::ConvId::Table {
            game: "test".into(),
        },
        id: "line".into(),
    };
    app.init_resource::<SocialHost>()
        .insert_resource(ChatUi {
            row_menu: Some(key.clone()),
            ..default()
        })
        .add_systems(Update, presses);
    let target = app
        .world_mut()
        .spawn((UiButton, RowPress::Whisper("0002".into())))
        .id();
    for action in [
        PointerAction::Press(PointerButton::Secondary),
        PointerAction::Release(PointerButton::Secondary),
    ] {
        testing::frame(&mut app, PointerId::Mouse, Some(target), &[action]);
        let ui = app.world().resource::<ChatUi>();
        assert_eq!(ui.row_menu.as_ref(), Some(&key));
        assert!(!ui.typing);
    }
    assert_eq!(app.world().resource::<testing::Activations>().0, [target]);
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(target),
        &[PointerAction::Press(PointerButton::Primary)],
    );
    assert_eq!(
        app.world().resource::<ChatUi>().row_menu.as_ref(),
        Some(&key)
    );
    assert!(
        !app.world().resource::<ChatUi>().typing,
        "the menu still owns the down"
    );
    testing::frame(
        &mut app,
        PointerId::Mouse,
        Some(target),
        &[PointerAction::Release(PointerButton::Primary)],
    );
    let ui = app.world().resource::<ChatUi>();
    assert!(ui.row_menu.is_none());
    assert!(ui.typing);
    assert_eq!(ui.field, Field::Composer);
    assert!(app.world().get::<bevy::ui::Pressed>(target).is_none());
}
