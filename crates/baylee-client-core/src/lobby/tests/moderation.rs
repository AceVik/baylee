//! What an admin's acts look like from the sign-in form (beta.8,
//! `docs/protocol.md` §"Acting on an account"): a password that was reset is
//! changed before anything else, a ban is said in its reason and its end,
//! and the gateway's `403`s are told apart by their bodies alone.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use crate::i18n::Lang;

/// The password an admin gave.
const GIVEN: &str = "7KQ2-M9XP-4TRW-C8HD";

/// Alice has typed the password the admin gave, and the gateway has said it
/// is a reset one.
fn reset() -> Lobby {
    let mut lobby = Lobby::new();
    lobby.set_field(Field::Username, "alice");
    lobby.set_field(Field::Password, GIVEN);
    assert!(matches!(lobby.submit(), Some(LobbyRequest::LogIn { .. })));
    assert_eq!(lobby.apply(LobbyEvent::PasswordChangeNeeded), None);
    lobby
}

#[test]
fn only_a_403_with_a_ban_or_a_reset_in_its_body_is_an_account_refusal() {
    assert_eq!(
        account_refusal(
            403,
            br#"{"error":"This account is banned. Reason: spam","ban":{"reason":"spam","until":null}}"#
        ),
        Some(LobbyEvent::Banned {
            reason: "spam".to_string(),
            until: None
        })
    );
    assert_eq!(
        account_refusal(
            403,
            br#"{"error":"x","ban":{"reason":"cheating","until":"2026-10-20T18:00:00Z"}}"#
        ),
        Some(LobbyEvent::Banned {
            reason: "cheating".to_string(),
            until: Some("2026-10-20T18:00:00Z".to_string())
        })
    );
    assert_eq!(
        account_refusal(403, br#"{"error":"x","password_change":true}"#),
        Some(LobbyEvent::PasswordChangeNeeded)
    );
    // A ban wins over a reset when a body says both.
    assert!(matches!(
        account_refusal(
            403,
            br#"{"password_change":true,"ban":{"reason":"r","until":null}}"#
        ),
        Some(LobbyEvent::Banned { .. })
    ));
    // Any other refusal is worded by the gateway and shown as it came.
    for (status, body) in [
        (403, &br#"{"error":"terms not accepted"}"#[..]),
        (403, br#"{"error":"x","password_change":false}"#),
        (403, b"not json"),
        (403, b""),
        (401, br#"{"error":"x","password_change":true}"#),
        (401, br#"{"error":"x","ban":{"reason":"r","until":null}}"#),
        (200, br#"{"ban":{"reason":"r","until":null}}"#),
        (503, br#"{"ban":{"reason":"r","until":null}}"#),
    ] {
        assert_eq!(
            account_refusal(status, body),
            None,
            "{status} {}",
            String::from_utf8_lossy(body)
        );
    }
}

#[test]
fn a_ban_ends_at_a_moment_a_player_can_read() {
    assert_eq!(ban_end("2026-10-20T18:00:00Z"), "2026-10-20 18:00 UTC");
    assert_eq!(ban_end("2999-01-01T00:00:59Z"), "2999-01-01 00:00 UTC");
    // What is not the gateway's shape is shown as it came.
    assert_eq!(ban_end("next week"), "next week");
    assert_eq!(
        ban_end("2026-10-20T18:00:00+02:00"),
        "2026-10-20T18:00:00+02:00"
    );
    assert_eq!(ban_end(""), "");
    assert_eq!(
        ban_end("ünïcödé-ünïcödé-ünïcödéZ"),
        "ünïcödé-ünïcödé-ünïcödéZ"
    );
}

#[test]
fn a_ban_is_said_with_its_reason_and_its_end_when_it_has_one() {
    assert_eq!(
        ban_sentence(Lang::En, "spam in names", None),
        "This account is banned. Reason: spam in names"
    );
    assert_eq!(
        ban_sentence(Lang::En, "cheating", Some("2026-10-20T18:00:00Z")),
        "This account is banned until 2026-10-20 18:00 UTC. Reason: cheating"
    );
    assert_eq!(
        ban_sentence(Lang::De, "Betrug", Some("2026-10-20T18:00:00Z")),
        "Dieser Account ist bis 2026-10-20 18:00 UTC gesperrt. Grund: Betrug"
    );
    assert_eq!(
        ban_sentence(Lang::De, "Betrug", None),
        "Dieser Account ist gesperrt. Grund: Betrug"
    );
}

#[test]
fn a_reset_password_opens_the_face_that_asks_for_a_new_one() {
    let mut lobby = Lobby::new();
    lobby.set_field(Field::Username, "alice");
    lobby.set_field(Field::Password, GIVEN);
    assert_eq!(lobby.face(), Face::SignIn);
    assert!(lobby.submit().is_some());
    assert!(lobby.busy());
    assert_eq!(lobby.apply(LobbyEvent::PasswordChangeNeeded), None);

    assert_eq!(lobby.face(), Face::NewPassword);
    assert_eq!(*lobby.screen(), Screen::SignIn { registering: false });
    assert!(!lobby.busy(), "the form can be used again");
    assert_eq!(
        lobby.field(Field::Password),
        "",
        "the boxes are for the new one"
    );
    assert_eq!(lobby.field(Field::PasswordAgain), "");
    assert_eq!(
        lobby.field(Field::Username),
        "alice",
        "who is signing in stays"
    );
    assert_eq!(lobby.focus(), Field::Password);
    assert_eq!(lobby.token(), None, "nobody is signed in");
}

#[test]
fn the_new_password_is_typed_twice_and_is_not_the_one_given() {
    let mut lobby = reset();

    // Nothing typed.
    assert_eq!(lobby.submit(), None);
    assert!(!lobby.busy());
    assert_eq!(lobby.tone(), Tone::Refusal);

    // Two that differ.
    lobby.set_field(Field::Password, "a-brand-new-one");
    lobby.set_field(Field::PasswordAgain, "a-brand-new-two");
    assert_eq!(lobby.submit(), None);
    assert_eq!(lobby.status(), "the two passwords differ");
    assert_eq!(lobby.tone(), Tone::Refusal);
    assert!(!lobby.busy());

    // The one the admin gave, typed twice.
    lobby.set_field(Field::Password, GIVEN);
    lobby.set_field(Field::PasswordAgain, GIVEN);
    assert_eq!(lobby.submit(), None, "no choice at all");
    assert_eq!(
        lobby.status(),
        "choose a password other than the one you were given"
    );
    assert_eq!(lobby.tone(), Tone::Refusal);
    assert!(!lobby.busy());
    assert_eq!(lobby.face(), Face::NewPassword);
}

/// The repeat box is on the new-password face, so the caret must be able to
/// enter it and a key must type into it; the username is not drawn there, so
/// it takes no typing. (`focus_on` and `typing_here` once treated
/// `Field::PasswordAgain` as hidden unless the form was creating an account.)
#[test]
fn the_caret_enters_and_types_into_the_repeat_box_of_the_new_password_face() {
    let mut lobby = reset();
    assert!(lobby.typing_here(), "the first box takes keys");
    lobby.type_char('a');
    assert_eq!(lobby.field(Field::Password), "a");

    lobby.focus_on(Field::PasswordAgain);
    assert_eq!(
        lobby.focus(),
        Field::PasswordAgain,
        "a click on it moves the caret"
    );
    assert!(lobby.typing_here(), "and a key typed there is not lost");
    lobby.type_char('b');
    lobby.insert("cd");
    assert_eq!(lobby.field(Field::PasswordAgain), "bcd");
    assert_eq!(
        lobby.field(Field::Password),
        "a",
        "nothing strayed into the first box"
    );
    assert_eq!(lobby.field(Field::Username), "alice");

    // Tab walks the two boxes there are.
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::Password);
    lobby.cycle_focus(Tab::Next);
    assert_eq!(lobby.focus(), Field::PasswordAgain);

    // The username is not drawn on this face: no key goes into it.
    lobby.focus_on(Field::Username);
    assert!(!lobby.typing_here());
}

/// After a mismatch the caret goes to the box to retype.
#[test]
fn a_mismatch_puts_the_caret_in_the_repeat_box() {
    let mut lobby = reset();
    lobby.set_field(Field::Password, "a-brand-new-one");
    lobby.set_field(Field::PasswordAgain, "a-brand-new-two");
    lobby.focus_on(Field::Password);
    assert_eq!(lobby.submit(), None);
    assert_eq!(lobby.focus(), Field::PasswordAgain);
    assert!(lobby.typing_here());
}

#[test]
fn a_valid_choice_is_sent_with_the_given_password_and_signs_in() {
    let mut lobby = reset();
    lobby.set_field(Field::Password, "a-brand-new-one");
    lobby.set_field(Field::PasswordAgain, "a-brand-new-one");
    assert_eq!(
        lobby.submit(),
        Some(LobbyRequest::ChangePassword {
            username: "alice".to_string(),
            password: GIVEN.to_string(),
            new_password: "a-brand-new-one".to_string(),
        })
    );
    assert!(lobby.busy());
    assert_eq!(lobby.submit(), None, "one at a time");

    assert_eq!(
        lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
            username: None
        }),
        Some(LobbyRequest::ListDecks)
    );
    assert_eq!(*lobby.screen(), Screen::Table);
    assert_eq!(lobby.token(), Some("tok"));
    assert_eq!(lobby.field(Field::Password), "", "no password is kept");
    assert_eq!(lobby.field(Field::PasswordAgain), "");
    assert_eq!(
        lobby.face(),
        Face::SignIn,
        "the given password is forgotten"
    );
}

#[test]
fn a_refused_choice_keeps_the_face_and_the_given_password() {
    let mut lobby = reset();
    lobby.set_field(Field::Password, "short");
    lobby.set_field(Field::PasswordAgain, "short");
    assert!(matches!(
        lobby.submit(),
        Some(LobbyRequest::ChangePassword { .. })
    ));
    lobby.apply(LobbyEvent::Failed("invalid password".to_string()));
    assert_eq!(lobby.face(), Face::NewPassword);
    assert_eq!(lobby.status(), "invalid password");
    // Tried again with a better one, still with what the admin gave.
    lobby.set_field(Field::Password, "a-brand-new-one");
    lobby.set_field(Field::PasswordAgain, "a-brand-new-one");
    assert_eq!(
        lobby.submit(),
        Some(LobbyRequest::ChangePassword {
            username: "alice".to_string(),
            password: GIVEN.to_string(),
            new_password: "a-brand-new-one".to_string(),
        })
    );
}

#[test]
fn back_leaves_the_new_password_face_and_forgets_the_given_password() {
    let mut lobby = reset();
    assert!(lobby.back_to_sign_in());
    assert_eq!(lobby.face(), Face::SignIn);
    assert_eq!(*lobby.screen(), Screen::SignIn { registering: false });
    assert!(!lobby.back_to_sign_in(), "already there");
    // A sign-in from here is a sign-in again, not a password change.
    lobby.set_field(Field::Password, "whatever");
    assert!(matches!(lobby.submit(), Some(LobbyRequest::LogIn { .. })));
}

#[test]
fn the_new_password_boxes_hide_what_is_typed_and_ask_for_a_new_password() {
    let lobby = reset();
    assert_eq!(lobby.field_kind(Field::Password), FieldKind::NewPassword);
    assert_eq!(
        lobby.field_kind(Field::PasswordAgain),
        FieldKind::NewPassword
    );
    let mut plain = Lobby::new();
    assert_eq!(plain.field_kind(Field::Password), FieldKind::Password);
    plain.toggle_registering();
    assert_eq!(plain.field_kind(Field::Password), FieldKind::NewPassword);
}

#[test]
fn a_ban_on_a_sign_in_says_why_and_signs_nobody_in() {
    let mut lobby = Lobby::new();
    lobby.set_field(Field::Username, "alice");
    lobby.set_field(Field::Password, "hunter22");
    assert!(lobby.submit().is_some());
    assert_eq!(
        lobby.apply(LobbyEvent::Banned {
            reason: "cheating".to_string(),
            until: Some("2026-10-20T18:00:00Z".to_string()),
        }),
        None
    );
    assert_eq!(*lobby.screen(), Screen::SignIn { registering: false });
    assert_eq!(
        lobby.status(),
        "This account is banned until 2026-10-20 18:00 UTC. Reason: cheating"
    );
    assert_eq!(lobby.tone(), Tone::Refusal);
    assert!(!lobby.busy());
    assert_eq!(lobby.token(), None);
    assert_eq!(lobby.face(), Face::SignIn);
}

#[test]
fn a_ban_met_while_signed_in_signs_the_account_out_with_the_reason() {
    let mut lobby = seated_lobby();
    assert_eq!(lobby.token(), Some("tok"));
    lobby.apply(LobbyEvent::Banned {
        reason: "spam in names".to_string(),
        until: None,
    });
    assert_eq!(lobby.token(), None, "the session is forgotten");
    assert_eq!(*lobby.screen(), Screen::SignIn { registering: false });
    assert!(
        lobby.status().contains("spam in names"),
        "the reason is shown: {}",
        lobby.status()
    );
    assert_eq!(
        lobby.status(),
        "This account is banned. Reason: spam in names"
    );
    assert_eq!(lobby.tone(), Tone::Refusal);
    assert!(lobby.decks().is_empty(), "what the account bought is gone");
}

#[test]
fn a_guest_that_is_banned_is_signed_out_but_stays_kept() {
    let mut lobby = Lobby::new();
    lobby.set_guests_enabled(true);
    let kept = KeptGuest {
        token: "guest-tok".to_string(),
        handle: "Casper#0007".to_string(),
    };
    lobby.keep_guest(Some(kept.clone()));
    lobby.play_as_guest();
    assert!(lobby.guest());
    lobby.apply(LobbyEvent::Banned {
        reason: "spam in names".to_string(),
        until: Some("2026-10-20T18:00:00Z".to_string()),
    });
    assert!(!lobby.guest());
    assert_eq!(lobby.token(), None);
    assert_eq!(
        lobby.kept_guest(),
        Some(&kept),
        "the guest is good again when the ban ends"
    );
    assert!(
        lobby.status().contains("spam in names"),
        "{}",
        lobby.status()
    );
    assert_eq!(*lobby.screen(), Screen::SignIn { registering: false });
}
