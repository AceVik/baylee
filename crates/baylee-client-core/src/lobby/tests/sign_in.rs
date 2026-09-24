//! The form as a statement: which boxes a log-in and a sign-up each have to hold before `submit` produces a request, the sign-up that chains into a log-in because the gateway hands out no token, and the chain that ends with the decks and the tables asked for and the password dropped. The account's other end is here too — a gateway that takes no new accounts withdraws the offer rather than leaving it dangling, and a sign-out forgets the token and everything it bought. The caret that moves between those boxes, the Tab ring and the keyboard each one asks for are `fields`; the line the form writes underneath itself is `status`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn a_sign_in_needs_both_fields() {
    let mut lobby = Lobby::new();
    assert_eq!(lobby.submit(), None);
    assert!(!lobby.busy());
    lobby.type_char('a');
    assert_eq!(lobby.submit(), None, "a password is still missing");
    lobby.focus_on(Field::Password);
    lobby.type_char('x');
    assert!(matches!(lobby.submit(), Some(LobbyRequest::LogIn { .. })));
}

#[test]
fn registering_also_needs_a_display_name() {
    let mut lobby = Lobby::new();
    lobby.toggle_registering();
    lobby.focus_on(Field::Username);
    lobby.insert("alice");
    lobby.focus_on(Field::Password);
    lobby.type_char('x');
    lobby.focus_on(Field::PasswordAgain);
    lobby.type_char('x');
    assert_eq!(lobby.submit(), None);
    assert_eq!(lobby.status(), "a display name, please");
    lobby.focus_on(Field::DisplayName);
    lobby.type_char('V');
    assert!(matches!(
        lobby.submit(),
        Some(LobbyRequest::Register { .. })
    ));
}

#[test]
fn registering_needs_the_password_typed_the_same_twice() {
    let mut lobby = Lobby::new();
    lobby.toggle_registering();
    lobby.insert("alice");
    lobby.focus_on(Field::DisplayName);
    lobby.type_char('V');
    lobby.focus_on(Field::Password);
    lobby.insert("hunter22");
    lobby.focus_on(Field::PasswordAgain);
    lobby.insert("hunter2 ");
    assert_eq!(lobby.submit(), None, "a space is a different password");
    assert_eq!(lobby.status(), "the two passwords differ");
    assert_eq!(lobby.tone(), Tone::Refusal);
    assert!(!lobby.busy());
    assert_eq!(
        lobby.focus(),
        Field::PasswordAgain,
        "the caret goes to the box to retype"
    );
    // The whole repeat is selected, so typing replaces it.
    lobby.insert("hunter22");
    assert_eq!(lobby.field(Field::PasswordAgain), "hunter22");
    assert_eq!(
        lobby.submit(),
        Some(LobbyRequest::Register {
            username: "alice".to_string(),
            display_name: "V".to_string(),
            password: "hunter22".to_string(),
        })
    );
}

#[test]
fn signing_in_asks_for_the_password_once() {
    let mut lobby = Lobby::new();
    lobby.type_char('a');
    lobby.focus_on(Field::PasswordAgain);
    assert_eq!(
        lobby.focus(),
        Field::Username,
        "the repeat is not drawn on the sign-in form, so the caret cannot go there"
    );
    lobby.focus_on(Field::Password);
    lobby.type_char('x');
    assert!(matches!(lobby.submit(), Some(LobbyRequest::LogIn { .. })));
}

#[test]
fn a_sign_up_chains_into_a_log_in() {
    let mut lobby = Lobby::new();
    lobby.toggle_registering();
    lobby.insert("alice");
    lobby.focus_on(Field::DisplayName);
    lobby.type_char('V');
    lobby.focus_on(Field::Password);
    lobby.type_char('x');
    lobby.focus_on(Field::PasswordAgain);
    lobby.type_char('x');
    assert!(matches!(
        lobby.submit(),
        Some(LobbyRequest::Register { .. })
    ));
    assert_eq!(
        lobby.apply(LobbyEvent::Registered),
        Some(LobbyRequest::LogIn {
            username: "alice".to_string(),
            password: "x".to_string(),
        }),
        "the gateway hands out no token on sign-up"
    );
    assert!(lobby.busy(), "the chained log-in is in flight");
    lobby.apply(LobbyEvent::LoggedIn {
        token: "tok".to_string(),
        username: None,
    });
    assert_eq!(
        (
            lobby.field(Field::Password),
            lobby.field(Field::PasswordAgain)
        ),
        ("", ""),
        "both copies of the password are dropped once it is spent"
    );
}

/// A name the rule refuses is refused here, before it is sent, in words
/// that say what to fix; the gateway checks again with the same rule.
#[test]
fn a_username_the_rule_refuses_is_named_before_it_is_sent() {
    for (typed, said) in [
        ("al", "a username has 3 to 24 characters"),
        (
            "alice smith",
            "a username is letters A–Z, digits and _ - . only",
        ),
        (
            "_alice",
            "a username starts and ends with a letter or a digit",
        ),
        ("al..ice", "a username has no two of _ - . in a row"),
        (
            "al\u{200b}ice",
            "that username holds a character that cannot be seen",
        ),
    ] {
        let mut lobby = Lobby::new();
        lobby.toggle_registering();
        lobby.insert(typed);
        lobby.focus_on(Field::DisplayName);
        lobby.type_char('V');
        lobby.focus_on(Field::Password);
        lobby.insert("hunter22");
        lobby.focus_on(Field::PasswordAgain);
        lobby.insert("hunter22");
        assert_eq!(lobby.submit(), None, "{typed:?} is not sent");
        assert_eq!(lobby.status(), said, "{typed:?}");
        assert_eq!(lobby.tone(), Tone::Refusal);
        assert_eq!(lobby.focus(), Field::Username, "the caret goes to the name");
        assert!(!lobby.busy());
    }
}

/// What is sent is the name in the form the rule keeps: a full-width name is
/// the letters it stands for.
#[test]
fn a_username_is_sent_in_the_form_the_rule_keeps() {
    let mut lobby = Lobby::new();
    lobby.toggle_registering();
    lobby.insert(" Ａｌｉｃｅ ");
    lobby.focus_on(Field::DisplayName);
    lobby.type_char('V');
    lobby.focus_on(Field::Password);
    lobby.insert("hunter22");
    lobby.focus_on(Field::PasswordAgain);
    lobby.insert("hunter22");
    assert_eq!(
        lobby.submit(),
        Some(LobbyRequest::Register {
            username: "Alice".to_string(),
            display_name: "V".to_string(),
            password: "hunter22".to_string(),
        })
    );
}

/// A sign-in is not checked against the rule: an address still signs in
/// until the end of 2026 (#269), and the gateway answers either with the
/// same refusal.
#[test]
fn a_sign_in_sends_what_was_typed() {
    let mut lobby = Lobby::new();
    lobby.insert(" mail@acevik.de ");
    lobby.focus_on(Field::Password);
    lobby.insert("hunter22");
    assert_eq!(
        lobby.submit(),
        Some(LobbyRequest::LogIn {
            username: "mail@acevik.de".to_string(),
            password: "hunter22".to_string(),
        })
    );
}

/// Signed in with an address, the player is told the username the account
/// was given, and the box takes the name, so the next sign-in is by name and
/// hears only "signed in".
#[test]
fn a_sign_in_by_address_is_told_the_name_to_use_instead() {
    let mut lobby = Lobby::new();
    lobby.insert("mail@acevik.de");
    lobby.focus_on(Field::Password);
    lobby.insert("hunter22");
    assert!(lobby.submit().is_some());
    lobby.apply(LobbyEvent::LoggedIn {
        token: "tok".to_string(),
        username: Some("mail".to_string()),
    });
    assert_eq!(
        lobby.status(),
        "signed in — your username is mail; the address works until the end of 2026"
    );
    assert_eq!(lobby.tone(), Tone::Note);
    assert_eq!(lobby.field(Field::Username), "mail");

    lobby.sign_out();
    lobby.focus_on(Field::Password);
    lobby.insert("hunter22");
    assert_eq!(
        lobby.submit(),
        Some(LobbyRequest::LogIn {
            username: "mail".to_string(),
            password: "hunter22".to_string(),
        })
    );
    lobby.apply(LobbyEvent::LoggedIn {
        token: "tok".to_string(),
        username: Some("mail".to_string()),
    });
    assert_eq!(lobby.status(), "signed in", "told once");
}

/// Signed in by name in another case, the box takes the name as its owner
/// wrote it, which is the spelling the device remembers.
#[test]
fn a_sign_in_by_name_keeps_the_spelling_the_gateway_answers() {
    let mut lobby = Lobby::new();
    lobby.insert("alice.b");
    lobby.focus_on(Field::Password);
    lobby.insert("hunter22");
    assert!(lobby.submit().is_some());
    lobby.apply(LobbyEvent::LoggedIn {
        token: "tok".to_string(),
        username: Some("Alice.B".to_string()),
    });
    assert_eq!(lobby.status(), "signed in");
    assert_eq!(lobby.field(Field::Username), "Alice.B");
}

#[test]
fn signing_in_asks_for_decks_and_then_for_games() {
    let lobby = seated_lobby();
    assert_eq!(*lobby.screen(), Screen::Table);
    assert_eq!(lobby.token(), Some("tok"));
    assert_eq!(lobby.selected(), Some(0), "the only deck is picked for us");
    assert!(!lobby.busy(), "the chain ended");
}

#[test]
fn the_password_is_dropped_once_it_has_been_spent() {
    let lobby = seated_lobby();
    assert_eq!(lobby.field(Field::Password), "");
}

#[test]
fn a_gateway_that_takes_no_sign_ups_offers_none() {
    let mut lobby = Lobby::new();
    lobby.set_registration_enabled(false);
    lobby.toggle_registering();
    assert_eq!(lobby.screen(), &Screen::SignIn { registering: false });
    assert_eq!(lobby.status(), "this gateway is not taking new accounts");
}

#[test]
fn a_form_already_registering_survives_the_config_arriving_late() {
    let mut lobby = Lobby::new();
    lobby.toggle_registering();
    lobby.set_registration_enabled(false);
    assert_eq!(
        lobby.screen(),
        &Screen::SignIn { registering: false },
        "the offer is withdrawn, not left dangling"
    );
}

#[test]
fn signing_out_forgets_the_token_and_everything_it_bought() {
    let mut lobby = seated_lobby();
    lobby.sign_out();
    assert_eq!(lobby.token(), None);
    assert!(lobby.decks().is_empty());
    assert!(lobby.games().is_empty());
    assert_eq!(lobby.selected(), None);
    assert_eq!(lobby.screen(), &Screen::SignIn { registering: false });
    assert_eq!(lobby.refresh(), None, "no token, no requests");
}
