//! Playing as a guest (#269): the entry above the sign-in form's tabs, the
//! guest this device keeps for a gateway and comes back as, and the two
//! ways a guest ends — signed out, which ends it on the gateway too, and
//! lapsed, which the gateway says with a `401`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// A lobby at a gateway that takes guests.
fn welcoming() -> Lobby {
    let mut lobby = Lobby::new();
    lobby.set_guests_enabled(true);
    lobby
}

/// The guest a gateway handed out, as it is kept.
fn kept() -> KeptGuest {
    KeptGuest {
        token: "guest-tok".to_string(),
        handle: "Casper#0007".to_string(),
    }
}

#[test]
fn a_gateway_that_has_not_said_it_takes_guests_offers_none() {
    let mut lobby = Lobby::new();
    assert!(!lobby.guest_offered());
    assert_eq!(lobby.play_as_guest(), None);
    assert_eq!(lobby.status(), "this gateway takes no guests");
    assert_eq!(lobby.tone(), Tone::Refusal);
    lobby.focus_on(Field::GuestName);
    assert_eq!(lobby.focus(), Field::Username, "no box, no caret in it");
}

#[test]
fn a_new_guest_is_asked_for_under_the_name_typed_or_none() {
    let mut lobby = welcoming();
    assert_eq!(
        lobby.open_guest_face(),
        None,
        "the face opens; nothing is sent"
    );
    assert_eq!(lobby.face(), Face::Guest);
    assert_eq!(lobby.focus(), Field::GuestName, "the caret is in the name");
    lobby.insert(" Casper ");
    assert_eq!(
        lobby.play_as_guest(),
        Some(LobbyRequest::PlayAsGuest {
            display_name: Some("Casper".to_string()),
            invite_key: None,
        })
    );
    assert!(lobby.busy());
    assert_eq!(lobby.play_as_guest(), None, "one at a time");

    let mut lobby = welcoming();
    assert_eq!(
        lobby.play_as_guest(),
        Some(LobbyRequest::PlayAsGuest {
            display_name: None,
            invite_key: None,
        }),
        "nothing typed is the gateway's own name for a guest"
    );
}

#[test]
fn enter_in_the_guest_name_is_the_guest_button() {
    let mut lobby = welcoming();
    lobby.open_guest_face();
    lobby.insert("Casper");
    assert!(matches!(
        lobby.submit(),
        Some(LobbyRequest::PlayAsGuest { .. })
    ));
}

#[test]
fn a_new_guest_goes_to_the_tables_and_is_kept() {
    let mut lobby = welcoming();
    lobby.focus_on(Field::Password);
    lobby.insert("typed-before-changing-my-mind");
    assert!(lobby.play_as_guest().is_some());
    assert_eq!(
        lobby.apply(LobbyEvent::GuestIn(kept())),
        Some(LobbyRequest::ListDecks)
    );
    assert_eq!(*lobby.screen(), Screen::Table);
    assert!(lobby.guest());
    assert_eq!(lobby.token(), Some("guest-tok"));
    assert_eq!(lobby.kept_guest(), Some(&kept()));
    assert_eq!(lobby.status(), "playing as a guest");
    assert_eq!(lobby.field(Field::Password), "", "a password is dropped");
}

/// The guest this device keeps is played as again with the session it has:
/// straight to the tables, and no name asked for.
#[test]
fn a_kept_guest_comes_back_without_asking_the_gateway_first() {
    let mut lobby = welcoming();
    lobby.keep_guest(Some(kept()));
    assert!(lobby.guest_offered());
    assert!(!lobby.guest_name_offered(), "its name is already its own");
    assert_eq!(lobby.play_as_guest(), Some(LobbyRequest::ListDecks));
    assert!(lobby.guest());
    assert_eq!(lobby.token(), Some("guest-tok"));
    assert_eq!(*lobby.screen(), Screen::Table);
}

#[test]
fn signing_out_ends_the_session_on_the_gateway_and_a_guest_with_it() {
    let mut account = seated_lobby();
    account.keep_guest(Some(kept()));
    assert_eq!(
        account.sign_out(),
        Some(LobbyRequest::LogOut {
            token: "tok".to_string()
        })
    );
    assert_eq!(
        account.kept_guest(),
        Some(&kept()),
        "an account signing out leaves the guest kept here alone"
    );

    let mut guest = welcoming();
    guest.keep_guest(Some(kept()));
    guest.play_as_guest();
    assert_eq!(
        guest.sign_out(),
        Some(LobbyRequest::LogOut {
            token: "guest-tok".to_string()
        })
    );
    assert!(!guest.guest());
    assert_eq!(guest.token(), None);
    assert_eq!(guest.kept_guest(), None, "a guest signed out is gone");

    let mut offline = Lobby::new();
    offline.play_offline();
    assert_eq!(offline.sign_out(), None, "offline, nobody to tell");
}

#[test]
fn a_guest_whose_session_ended_is_let_go_and_the_player_told() {
    let mut guest = welcoming();
    guest.apply(LobbyEvent::GuestIn(kept()));
    guest.session_ended();
    assert_eq!(guest.kept_guest(), None);
    assert_eq!(guest.token(), None);
    assert_eq!(
        guest.status(),
        "that guest has ended — play as a new one, or sign in"
    );
    assert_eq!(guest.tone(), Tone::Refusal);
    assert!(
        guest.guest_offered() && guest.kept_guest().is_none(),
        "the next one is a new one"
    );

    // An account is signed out and told why, in the shell design's words
    // (§2.5): a session that ended is not a sign-out the player chose.
    let mut account = seated_lobby();
    account.session_ended();
    assert_eq!(
        account.status(),
        "Signed out \u{2014} your session ended · Sign in again"
    );
    assert_eq!(account.tone(), Tone::Note);
}

/// The guest's face (WP1) is a ring of its own: the name alone on an open
/// gateway; the sign-in face never draws it.
#[test]
fn the_guests_name_is_a_ring_of_its_own_on_the_guest_face() {
    let mut lobby = welcoming();
    lobby.focus_on(Field::GuestName);
    assert_eq!(
        lobby.focus(),
        Field::Username,
        "not drawn on the sign-in face"
    );
    lobby.open_guest_face();
    assert_eq!(lobby.focus(), Field::GuestName);
    assert!(lobby.typing_here());
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::GuestName, "a ring of one");
    assert!(lobby.back_to_sign_in());
    assert_eq!(lobby.face(), Face::SignIn);
    assert_eq!(
        lobby.focus(),
        Field::Username,
        "the box went, and the caret"
    );
    lobby.cycle_focus(Tab::Back);
    lobby.cycle_focus(Tab::Back);
    assert_eq!(lobby.focus(), Field::Username, "two round, without it");
    assert!(!lobby.back_to_sign_in(), "already there");
}

/// A kept guest needs no face: the sign-in face's button plays at once.
#[test]
fn a_kept_guest_plays_from_the_sign_in_face_without_a_face_of_its_own() {
    let mut lobby = welcoming();
    lobby.keep_guest(Some(kept()));
    assert_eq!(lobby.open_guest_face(), Some(LobbyRequest::ListDecks));
    assert_eq!(*lobby.screen(), Screen::Table);
}

/// A parked caret types nowhere, and placing it again ends the parking.
#[test]
fn a_parked_caret_types_nowhere_until_a_field_is_focused() {
    let mut lobby = welcoming();
    lobby.focus_on(Field::Username);
    lobby.park_caret();
    assert!(lobby.caret_parked() && !lobby.typing_here());
    assert!(!lobby.caret_in(Field::Username));
    lobby.focus_on(Field::Password);
    assert!(!lobby.caret_parked() && lobby.typing_here());
    assert!(lobby.caret_in(Field::Password));
}

#[test]
fn a_kept_guests_token_is_never_printed() {
    let printed = format!("{:?}", kept());
    assert!(!printed.contains("guest-tok"), "{printed}");
    assert!(printed.contains("Casper#0007"), "{printed}");
}
