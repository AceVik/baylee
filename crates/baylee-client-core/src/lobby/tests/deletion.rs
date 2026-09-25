//! Deleting the signed-in account (#292): `docs/protocol.md` §"Deleting an
//! account (#292)" is the gateway's half.

use super::*;

/// A guest signed in on a gateway that takes them.
fn a_guest() -> Lobby {
    let mut lobby = Lobby::new();
    lobby.set_guests_enabled(true);
    lobby.keep_guest(Some(KeptGuest {
        token: "guest-tok".to_string(),
        handle: "Casper#0007".to_string(),
    }));
    lobby.play_as_guest();
    assert!(lobby.guest());
    // Its decks and the tables, answered, so nothing is in flight.
    lobby.apply(LobbyEvent::Decks(Vec::new()));
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    assert!(!lobby.busy());
    lobby
}

/// Types `text` the way a player does, into whichever box has the caret.
fn type_in(lobby: &mut Lobby, text: &str) {
    assert!(lobby.typing_here(), "the caret is in no box on show");
    for ch in text.chars() {
        lobby.type_char(ch);
    }
}

/// A registered account is asked its password again, typed into the
/// confirmation's own box, and nothing is sent without it.
#[test]
fn an_account_is_deleted_with_its_password_typed_again() {
    let mut lobby = seated_lobby();
    lobby.ask_to_delete_account();
    assert!(lobby.deleting_account().is_some());
    assert_eq!(lobby.focus(), Field::AccountPassword);
    assert_eq!(lobby.account_name(), "alice");

    assert_eq!(lobby.submit(), None, "sent with no password typed");
    assert_eq!(
        lobby.deleting_account().and_then(|d| d.refusal.as_deref()),
        Some("your password, please")
    );

    type_in(&mut lobby, "hunter22");
    assert_eq!(
        lobby.field(Field::Password),
        "",
        "the sign-in form's box took the password"
    );
    assert_eq!(
        lobby.submit(),
        Some(LobbyRequest::DeleteAccount {
            password: Some("hunter22".to_string())
        })
    );
    assert!(lobby.busy());
    assert_eq!(lobby.delete_account(), None, "a second press sends twice");
}

/// A wrong password, or too many tries, leaves the session good: the
/// confirmation stays up with the gateway's words, and the typed password
/// goes.
#[test]
fn a_refused_deletion_keeps_the_account_and_says_why() {
    let mut lobby = seated_lobby();
    lobby.ask_to_delete_account();
    type_in(&mut lobby, "wrong");
    assert!(lobby.delete_account().is_some());
    lobby.apply(LobbyEvent::Failed("wrong password".to_string()));

    assert_eq!(
        lobby.token(),
        Some("tok"),
        "a refusal signed the account out"
    );
    assert_eq!(*lobby.screen(), Screen::Table);
    let deletion = lobby.deleting_account().expect("the confirmation closed");
    assert_eq!(deletion.refusal.as_deref(), Some("wrong password"));
    assert_eq!(lobby.field(Field::AccountPassword), "");
}

/// A `204` is the account gone: the session is forgotten and the lobby is
/// back at the front door, saying so.
#[test]
fn a_deleted_account_is_signed_out_of_here() {
    let mut lobby = seated_lobby();
    lobby.ask_to_delete_account();
    type_in(&mut lobby, "hunter22");
    assert!(lobby.delete_account().is_some());
    assert_eq!(lobby.apply(LobbyEvent::AccountDeleted), None);

    assert_eq!(lobby.token(), None);
    assert_eq!(*lobby.screen(), Screen::SignIn { registering: false });
    assert!(lobby.deleting_account().is_none());
    assert_eq!(lobby.field(Field::AccountPassword), "");
    assert_eq!(lobby.status(), "account deleted");
}

/// A guest has no password: its confirmation sends at once, and a guest
/// deleted is not kept for the next visit.
#[test]
fn a_guest_is_deleted_on_its_session_alone() {
    let mut lobby = a_guest();
    lobby.ask_to_delete_account();
    assert_eq!(lobby.account_name(), "Casper#0007");
    assert_eq!(
        lobby.submit(),
        Some(LobbyRequest::DeleteAccount { password: None })
    );
    lobby.apply(LobbyEvent::AccountDeleted);
    assert!(!lobby.guest());
    assert_eq!(lobby.kept_guest(), None, "the deleted guest is still kept");
}

/// Cancelling deletes nothing, drops what was typed and gives the caret
/// back; a session that expires under the confirmation closes it.
#[test]
fn a_cancelled_or_expired_deletion_leaves_nothing_behind() {
    let mut lobby = seated_lobby();
    let before = lobby.focus();
    lobby.ask_to_delete_account();
    type_in(&mut lobby, "hunter22");
    lobby.cancel_account_deletion();
    assert!(lobby.deleting_account().is_none());
    assert_eq!(lobby.focus(), before);
    assert_eq!(lobby.field(Field::AccountPassword), "");
    assert_eq!(lobby.token(), Some("tok"));

    lobby.ask_to_delete_account();
    lobby.session_ended();
    assert!(lobby.deleting_account().is_none());

    let mut offline = Lobby::new();
    offline.play_offline();
    offline.ask_to_delete_account();
    assert!(
        offline.deleting_account().is_none(),
        "offline there is no account to delete"
    );
}
