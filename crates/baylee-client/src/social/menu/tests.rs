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

/// A guest's line offers neither a whisper nor a friend request, and says why
/// in their place; an account's offers both.
#[test]
fn a_guests_line_says_why_it_offers_no_way_to_reach_them() {
    let person = |guest| baylee_protocol::social::Person {
        tag: "1a2b".into(),
        handle: "Bob#1a2b".into(),
        guest,
    };
    let phrases = |ways: Vec<(ChatPhrase, RowPress)>| -> Vec<ChatPhrase> {
        ways.into_iter().map(|(phrase, _)| phrase).collect()
    };
    assert_eq!(
        phrases(ways_to_reach(&person(true), false, false)),
        vec![ChatPhrase::GuestNoContact]
    );
    assert_eq!(
        phrases(ways_to_reach(&person(false), false, false)),
        vec![ChatPhrase::WhisperButton, ChatPhrase::AddFriendButton]
    );
}
