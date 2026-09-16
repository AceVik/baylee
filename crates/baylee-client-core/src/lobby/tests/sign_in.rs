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
    lobby.focus_on(Field::Email);
    lobby.type_char('a');
    lobby.focus_on(Field::Password);
    lobby.type_char('x');
    assert_eq!(lobby.submit(), None);
    lobby.focus_on(Field::DisplayName);
    lobby.type_char('V');
    assert!(matches!(
        lobby.submit(),
        Some(LobbyRequest::Register { .. })
    ));
}

#[test]
fn a_sign_up_chains_into_a_log_in() {
    let mut lobby = Lobby::new();
    lobby.toggle_registering();
    lobby.type_char('a');
    lobby.focus_on(Field::DisplayName);
    lobby.type_char('V');
    lobby.focus_on(Field::Password);
    lobby.type_char('x');
    lobby.submit();
    assert_eq!(
        lobby.apply(LobbyEvent::Registered {
            confirmation_required: false,
        }),
        Some(LobbyRequest::LogIn {
            email: "a".to_string(),
            password: "x".to_string(),
        }),
        "the gateway hands out no token on sign-up"
    );
    assert!(lobby.busy(), "the chained log-in is in flight");
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
