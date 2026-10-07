//! A closed beta's key (#317): the box that asks for it, drawn only where a
//! new account or a new guest is to be made on a gateway that wants one, and
//! what goes out with the request.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// A key as the gateway's command prints it.
const KEY: &str = "BAYLEE-7K3M-Q9TX-2HVD-4WNC";

/// A lobby at a closed beta that takes guests, on the sign-in tab.
fn closed_beta() -> Lobby {
    let mut lobby = Lobby::new();
    lobby.set_registration(Registration::Invite);
    lobby.set_guests_enabled(true);
    lobby
}

/// The sign-up form, filled in but for the key.
fn signing_up(lobby: &mut Lobby) {
    lobby.toggle_registering();
    lobby.set_field(Field::Username, "alice");
    lobby.set_field(Field::DisplayName, "Alice");
    lobby.set_field(Field::Password, "hunter22");
    lobby.set_field(Field::PasswordAgain, "hunter22");
}

/// What Tab visits, from the first field of the form round to it again.
fn ring(lobby: &mut Lobby) -> Vec<Field> {
    lobby.focus_on(Field::Username);
    let mut seen = vec![Field::Username];
    loop {
        lobby.cycle_focus(Tab::Next);
        if lobby.focus() == Field::Username {
            return seen;
        }
        seen.push(lobby.focus());
        assert!(seen.len() < 12, "Tab never came back: {seen:?}");
    }
}

#[test]
fn what_a_gateway_says_about_registration_is_read_three_ways() {
    for (enabled, said, read) in [
        (true, None, Registration::Open),
        (false, None, Registration::Off),
        (true, Some("open"), Registration::Open),
        (true, Some("invite"), Registration::Invite),
        (false, Some("invite"), Registration::Invite),
        (true, Some("off"), Registration::Off),
        (false, Some("open"), Registration::Open),
        // A word from a gateway newer than this client: the old flag decides.
        (true, Some("waitlist"), Registration::Open),
        (false, Some("waitlist"), Registration::Off),
    ] {
        assert_eq!(
            Registration::read(enabled, said),
            read,
            "{enabled} {said:?}"
        );
    }
}

#[test]
fn the_key_is_asked_for_only_by_a_closed_beta_with_something_to_make() {
    // An open gateway, and one from before keys: no box, on either tab.
    let mut open = Lobby::new();
    open.set_guests_enabled(true);
    assert!(!open.invite_key_offered());
    open.toggle_registering();
    assert!(!open.invite_key_offered());
    open.focus_on(Field::InviteKey);
    assert_ne!(open.focus(), Field::InviteKey, "the caret stays out of it");
    assert!(!ring(&mut open).contains(&Field::InviteKey));

    // A closed beta: the box, last in the ring, on the create-account
    // face (WP1: the key opens under the button chosen) …
    let mut lobby = closed_beta();
    lobby.toggle_registering();
    assert!(lobby.invite_key_offered());
    assert_eq!(
        ring(&mut lobby),
        [
            Field::Username,
            Field::DisplayName,
            Field::Password,
            Field::PasswordAgain,
            Field::InviteKey,
        ],
        "the account's fields, then the key under them"
    );
    lobby.focus_on(Field::InviteKey);
    assert!(lobby.typing_here());

    // … never on the sign-in face …
    lobby.toggle_registering();
    assert!(!lobby.invite_key_offered());
    assert_ne!(lobby.focus(), Field::InviteKey, "the caret left the box");
    assert_eq!(ring(&mut lobby), [Field::Username, Field::Password]);

    // … and on the guest's face for a new guest …
    lobby.open_guest_face();
    assert!(lobby.invite_key_offered());
    assert_eq!(lobby.focus(), Field::GuestName);
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::InviteKey);
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::GuestName, "name, key, round");

    // … but not for a guest this device keeps, nor with no guests at all.
    lobby.keep_guest(Some(KeptGuest {
        token: "tok".into(),
        handle: "Casper#0007".into(),
    }));
    assert!(!lobby.invite_key_offered());
    assert_ne!(lobby.focus(), Field::InviteKey, "the caret left the box");
    let mut no_guests = Lobby::new();
    no_guests.set_registration(Registration::Invite);
    assert!(!no_guests.invite_key_offered());
    no_guests.toggle_registering();
    assert!(
        no_guests.invite_key_offered(),
        "the sign-up tab still has it"
    );

    // Leaving the sign-up tab takes the caret out of a box no longer drawn.
    no_guests.focus_on(Field::InviteKey);
    no_guests.toggle_registering();
    assert_eq!(no_guests.focus(), Field::Username);
}

#[test]
fn a_closed_beta_sign_up_without_a_key_is_stopped_here() {
    let mut lobby = closed_beta();
    signing_up(&mut lobby);
    assert_eq!(lobby.submit(), None);
    assert_eq!(lobby.status(), "your closed beta key, please");
    assert_eq!(lobby.tone(), Tone::Refusal);
    assert_eq!(
        lobby.focus(),
        Field::InviteKey,
        "the caret goes where it is missing"
    );
    assert!(!lobby.busy(), "nothing was sent");

    lobby.set_field(Field::InviteKey, "   ");
    assert_eq!(lobby.submit(), None, "blank is none");
}

#[test]
fn a_closed_beta_sign_up_sends_the_key_as_it_was_pasted() {
    let mut lobby = closed_beta();
    signing_up(&mut lobby);
    lobby.focus_on(Field::InviteKey);
    // A key copied out of a message comes with the line it ended.
    lobby.insert(&format!("  {KEY}\n"));
    assert_eq!(lobby.field(Field::InviteKey), format!("  {KEY}"));
    assert_eq!(
        lobby.submit(),
        Some(LobbyRequest::Register {
            username: "alice".to_string(),
            display_name: "Alice".to_string(),
            password: "hunter22".to_string(),
            invite_key: Some(KEY.to_string()),
        })
    );
    // The account is made: the key is spent, and not kept for the next one.
    assert!(matches!(
        lobby.apply(LobbyEvent::Registered),
        Some(LobbyRequest::LogIn { .. })
    ));
    assert_eq!(lobby.field(Field::InviteKey), "");
}

#[test]
fn an_open_gateway_is_sent_no_key() {
    let mut lobby = Lobby::new();
    signing_up(&mut lobby);
    lobby.set_field(Field::InviteKey, KEY);
    assert!(matches!(
        lobby.submit(),
        Some(LobbyRequest::Register {
            invite_key: None,
            ..
        })
    ));
}

#[test]
fn a_new_guest_of_a_closed_beta_brings_the_key_and_a_kept_one_does_not() {
    let mut lobby = closed_beta();
    assert_eq!(lobby.play_as_guest(), None);
    assert_eq!(lobby.status(), "your closed beta key, please");
    assert!(!lobby.busy());

    lobby.open_guest_face();
    lobby.focus_on(Field::InviteKey);
    lobby.insert(KEY);
    // Enter anywhere on the guest's face is its button.
    assert_eq!(
        lobby.submit(),
        Some(LobbyRequest::PlayAsGuest {
            display_name: None,
            invite_key: Some(KEY.to_string()),
        })
    );
    lobby.apply(LobbyEvent::GuestIn(KeptGuest {
        token: "tok".into(),
        handle: "Guest#0001".into(),
    }));
    assert_eq!(lobby.field(Field::InviteKey), "", "spent");

    // Back at the door with the guest kept: no key, the session is enough.
    let mut lobby = closed_beta();
    lobby.keep_guest(Some(KeptGuest {
        token: "tok".into(),
        handle: "Guest#0001".into(),
    }));
    assert_eq!(lobby.play_as_guest(), Some(LobbyRequest::ListDecks));
}

#[test]
fn a_paste_is_one_line() {
    let mut lobby = closed_beta();
    lobby.open_guest_face();
    lobby.focus_on(Field::InviteKey);
    lobby.insert("BAYLEE-\r\n7K3M-\tQ9TX-2HVD-4WNC\u{7}");
    assert_eq!(lobby.field(Field::InviteKey), "BAYLEE-7K3M-Q9TX-2HVD-4WNC");
}
