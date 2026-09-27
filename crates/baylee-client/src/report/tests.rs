//! The report form in the shell: what it collects, and the Bevy side of it.
//!
//! What a report may carry, what each answer of the gateway means and how
//! a paste is cut are client-core's (`bugreport`) and tested there. Here is
//! what only the shell can get wrong: which tokens it hands `seal`, and
//! (in `desk`) the form's keys, buttons and the crash question running in
//! an `App`.

use baylee_client_core::bugreport::Keyring;
use baylee_client_core::lobby::{KeptGuest, LobbyEvent, SeatHandover};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::PlayerAction;

use super::*;
use crate::host::{DuelHost, HostMessage};


pub(super) const SESSION: &str = "5e55105e55105e55105e55105e55105e";
pub(super) const AWAITED_SEAT: &str = "a3a17ed5ea7a3a17ed5ea7a3a17ed5ea";
pub(super) const TABLE_SEAT: &str = "7ab1e5ea77ab1e5ea77ab1e5ea77ab1e";
pub(super) const GUEST: &str = "9ue579ue579ue579ue579ue579ue579u";

/// A table's host that holds a seat token, as the networked one does. A
/// `NetworkHost` cannot be built without dialling; `tests/network_host.rs`
/// checks that the real one answers with its ticket's token.
pub(super) struct Seated(pub(super) &'static str);

impl DuelHost for Seated {
    fn poll(&mut self) -> Vec<HostMessage> {
        Vec::new()
    }
    fn submit(&mut self, _: PlayerAction) {}
    fn ready(&mut self) {}
    fn seat(&self) -> PlayerId {
        PlayerId::new(0)
    }
    fn seat_token(&self) -> Option<&str> {
        Some(self.0)
    }
}

/// A lobby signed in with [`SESSION`], its gateway an address no request
/// can be built from.
pub(super) fn signed_in() -> LobbyState {
    let mut state = LobbyState::new();
    state.gateway = "http://127.0.0.1:1".into();
    state.lobby.apply(LobbyEvent::LoggedIn {
        token: SESSION.into(),
        username: Some("ada".into()),
    });
    state
}

/// A lobby that also holds a seat at a table still waiting for players.
fn awaiting() -> LobbyState {
    let mut state = signed_in();
    state.lobby.apply(LobbyEvent::Seated(SeatHandover {
        game_id: "g-1".into(),
        seat: 0,
        seat_token: AWAITED_SEAT.into(),
        ..SeatHandover::default()
    }));
    assert!(
        state.lobby.awaiting().is_some(),
        "the lobby holds the seat while the table waits"
    );
    state
}

/// Settings keeping [`GUEST`] for some gateway.
pub(super) fn with_guest() -> ClientSettings {
    let mut settings = ClientSettings::default();
    settings.guests.insert(
        "https://elsewhere.example".into(),
        KeptGuest {
            token: GUEST.into(),
            handle: "Guest#1a2b".into(),
        },
    );
    settings
}

/// The table's seat token is in the keyring while the table is up, when the
/// lobby no longer holds it: the gap #314 left, where a report written at
/// the table was checked against the session alone.
#[test]
fn the_table_s_seat_token_is_in_the_keyring_once_the_lobby_let_go_of_it() {
    let lobby = signed_in();
    assert!(lobby.lobby.awaiting().is_none());
    let host = crate::InstalledHost(Box::new(Seated(TABLE_SEAT)));
    let ring = keyring(Some(&lobby), Some(&host), None, None, &with_guest());
    assert_eq!(ring.seats, [TABLE_SEAT]);
    assert_eq!(ring.sessions, [SESSION]);
    assert_eq!(ring.guests, [GUEST]);
}

/// And the lobby's, while it holds one: a table still waiting for players.
#[test]
fn a_seat_the_lobby_holds_is_in_the_keyring() {
    let lobby = awaiting();
    let ring = keyring(Some(&lobby), None, None, None, &ClientSettings::default());
    assert_eq!(ring.seats, [AWAITED_SEAT]);
}

/// And the one a lobby is taking to the table: between the gateway's
/// answer and the host being installed it is on the lobby's screen.
#[test]
fn a_seat_on_its_way_to_the_table_is_in_the_keyring() {
    let mut lobby = signed_in();
    lobby
        .lobby
        .apply(LobbyEvent::Decks(vec![baylee_client_core::lobby::DeckSummary {
            id: "d1".into(),
            name: "Allytifact".into(),
            cards: 60,
            ..Default::default()
        }]));
    // The deck list asked for the table list; answered, the lobby is idle.
    lobby.lobby.apply(LobbyEvent::Games(
        baylee_client_core::lobby::GameListing::default(),
    ));
    assert!(
        lobby
            .lobby
            .host(baylee_client_core::lobby::GameMode::Ai)
            .is_some(),
        "the lobby asks for a table against the house"
    );
    lobby.lobby.apply(LobbyEvent::Seated(SeatHandover {
        game_id: "g-2".into(),
        seat: 0,
        seat_token: AWAITED_SEAT.into(),
        ..SeatHandover::default()
    }));
    assert!(matches!(lobby.lobby.screen(), Screen::Seated(_)));
    let ring = keyring(Some(&lobby), None, None, None, &ClientSettings::default());
    assert_eq!(ring.seats, [AWAITED_SEAT]);
}

/// Every copy of the session is in it: the preferences keep one and the
/// card text another, and either may outlive the lobby's.
#[test]
fn every_copy_of_the_session_is_in_the_keyring() {
    let mut prefs = crate::prefs::Prefs::default();
    prefs.attach("http://127.0.0.1:1", "0ld5e5510n0ld5e5510n0ld5e5510n00");
    let text = crate::cardtext::TextGateway(Some(crate::cardtext::SignedIn {
        base: "http://127.0.0.1:1".into(),
        token: "7ex75e5510n7ex75e5510n7ex75e5510".into(),
    }));
    let ring = keyring(
        None,
        None,
        Some(&prefs),
        Some(&text),
        &ClientSettings::default(),
    );
    assert_eq!(
        ring.sessions,
        [
            "0ld5e5510n0ld5e5510n0ld5e5510n00",
            "7ex75e5510n7ex75e5510n7ex75e5510"
        ]
    );
}

/// Each kind the shell collects refuses a report carrying it, named by its
/// label: the whole path from where the token lives to `seal`.
#[test]
fn a_report_carrying_any_token_the_client_holds_is_refused() {
    let lobby = awaiting();
    let host = crate::InstalledHost(Box::new(Seated(TABLE_SEAT)));
    let settings = with_guest();
    let ring = keyring(Some(&lobby), Some(&host), None, None, &settings);
    for (token, label) in [
        (SESSION, Keyring::SESSION),
        (AWAITED_SEAT, Keyring::SEAT),
        (TABLE_SEAT, Keyring::SEAT),
        (GUEST, Keyring::GUEST),
    ] {
        let mut form = ReportForm::default();
        form.paste(&format!("it said {token}"));
        let sent = form.prepare(&Gathered::default(), &Consent::default(), &ring.secrets());
        assert!(sent.is_none(), "{label} went out");
        assert_eq!(
            form.status,
            Status::Blocked(bugreport::Unsendable::Leaked(bugreport::Leaked {
                label: label.into()
            })),
        );
    }
}
