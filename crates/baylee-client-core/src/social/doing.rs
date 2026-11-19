//! What the client says it is doing, for Baylee Central's friends
//! (`docs/client.md` §"The contact status").
//!
//! Two things can be said: sitting in a game that is running, and sitting at
//! a table whose game has not begun. Watching a game, a replay, the deck
//! builder and a result screen are none of them: online. With either the
//! client may name the table ([`shared_table`]), while the player shares it
//! and it has no password; nothing else names a game or a gateway.

use central_api::{
    gateway::shown_char,
    presence::{Activity, Table},
};

use crate::lobby::{GameSummary, Lobby, Screen, strips::my_waiting_table};

/// What the client is doing: `screen` is the lobby's, `finished` whether
/// the duel on screen has ended (its result is shown), and `waiting` whether
/// this player has a chair at a table still waiting to start.
///
/// A game counts from the moment its seat is handed over, online or against
/// the house AI in this process, until its result stands.
#[must_use]
pub fn doing(screen: &Screen, finished: bool, waiting: bool) -> Option<Activity> {
    match screen {
        Screen::Seated(seat) if !seat.watching() && !finished => Some(Activity::Playing),
        Screen::Seated(_) | Screen::Replay(_) => None,
        Screen::SignIn { .. } | Screen::Table | Screen::Build => waiting.then_some(Activity::Table),
    }
}

/// The table this client may tell Baylee Central its player sits at
/// (`docs/client.md` §"The contact status"): the game it plays, or the table
/// it waits at, as the lobby `lobby` lists it at the gateway `gateway` (the
/// session's address). `None` wherever there is nothing it may name: no
/// activity ([`doing`]), a game in this process against the house AI, a game
/// the listing does not hold (so whether it has a password is not known), a
/// table with a password, or one it cannot name within Central's bounds
/// ([`Table::valid`]).
///
/// Whether the player shares its table at all is the gateway's setting
/// (`share_table`), which the caller checks before it sends this anywhere.
#[must_use]
pub fn shared_table(
    screen: &Screen,
    finished: bool,
    lobby: &Lobby,
    gateway: &str,
) -> Option<Table> {
    let waiting = my_waiting_table(lobby);
    let game = match doing(screen, finished, waiting.is_some())? {
        Activity::Playing => match screen {
            Screen::Seated(seat) if !seat.local => {
                lobby.games().iter().find(|game| game.id == seat.game_id)?
            }
            _ => return None,
        },
        Activity::Table => waiting?,
    };
    table_of(game, gateway)
}

/// `game` as a shared table at `gateway`; `None` for one with a password or
/// one out of Central's bounds.
fn table_of(game: &GameSummary, gateway: &str) -> Option<Table> {
    if game.locked {
        return None;
    }
    let table = Table {
        origin: origin_of(gateway)?,
        game: game.id.clone(),
        name: plain_name(&game.name),
        seats: u8::try_from(game.seats.len()).ok()?,
        taken: u8::try_from(game.seats.iter().filter(|seat| seat.taken).count()).ok()?,
    };
    table.valid().then_some(table)
}

/// A table's name as Central takes it: without the characters that may not
/// stand in a name another person reads (a gateway lets some invisible ones
/// through), trimmed, at most [`Table::MAX_NAME_CHARS`] characters.
fn plain_name(name: &str) -> String {
    let shown: String = name.chars().filter(|c| shown_char(*c)).collect();
    shown.trim().chars().take(Table::MAX_NAME_CHARS).collect()
}

/// A gateway's address as its origin, spelled as a shared table names it and
/// as a friend's client compares it with its own: scheme, host and port,
/// lower case, without a path and without the scheme's default port. `None`
/// for an address that has no such origin.
#[must_use]
pub fn origin_of(gateway: &str) -> Option<String> {
    let origin = url::Url::parse(gateway).ok()?.origin();
    origin.is_tuple().then(|| origin.ascii_serialization())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lobby::SeatHandover;

    fn seat(seat: u32, local: bool) -> Screen {
        Screen::Seated(SeatHandover {
            game_id: "g".into(),
            seat,
            seat_token: String::new(),
            local,
        })
    }

    #[test]
    fn a_seat_in_a_running_game_plays_online_or_against_the_house() {
        for local in [false, true] {
            assert_eq!(
                doing(&seat(0, local), false, false),
                Some(Activity::Playing)
            );
            // The chair in the listing may still read as waiting while the
            // game opens: the game wins.
            assert_eq!(doing(&seat(1, local), false, true), Some(Activity::Playing));
        }
    }

    #[test]
    fn a_result_a_watched_game_and_a_replay_are_neither() {
        assert_eq!(doing(&seat(0, false), true, false), None);
        assert_eq!(doing(&seat(SeatHandover::WATCH, false), false, false), None);
        assert_eq!(doing(&seat(SeatHandover::WATCH, false), false, true), None);
    }

    #[test]
    fn a_chair_at_a_waiting_table_is_at_a_table_wherever_the_lobby_stands() {
        for screen in [Screen::Table, Screen::Build] {
            assert_eq!(doing(&screen, false, true), Some(Activity::Table));
            assert_eq!(doing(&screen, false, false), None);
        }
    }

    const GATEWAY: &str = "https://Gateway.Example:443/";
    const ORIGIN: &str = "https://gateway.example";

    /// A listed game with four chairs, `taken` of them taken, the first by
    /// this player when `mine`.
    fn listed(id: &str, name: &str, state: &str, taken: u32, mine: bool) -> GameSummary {
        GameSummary {
            id: id.into(),
            name: name.into(),
            state: state.into(),
            seats: (0..4)
                .map(|index| crate::lobby::GameSeat {
                    seat: index,
                    taken: index < taken,
                    you: mine && index == 0,
                    ..crate::lobby::GameSeat::default()
                })
                .collect(),
            ..GameSummary::default()
        }
    }

    fn listing(games: Vec<GameSummary>) -> Lobby {
        let mut lobby = Lobby::default();
        lobby.apply(crate::lobby::LobbyEvent::LoggedIn {
            token: "tok".into(),
            username: None,
        });
        lobby.apply(crate::lobby::LobbyEvent::Games(crate::lobby::GameListing {
            total: games.len(),
            games,
            ..crate::lobby::GameListing::default()
        }));
        lobby
    }

    fn shared(id: &str, name: &str, taken: u8) -> Table {
        Table {
            origin: ORIGIN.into(),
            game: id.into(),
            name: name.into(),
            seats: 4,
            taken,
        }
    }

    #[test]
    fn the_table_waited_at_or_played_is_named_with_the_gateway_s_origin() {
        let lobby = listing(vec![
            listed("other", "Not mine", "waiting", 1, false),
            listed("wait-1", "Friday draft", "waiting", 2, true),
            listed("g", "Running", "playing", 4, true),
        ]);
        assert_eq!(
            shared_table(&Screen::Table, false, &lobby, GATEWAY),
            Some(shared("wait-1", "Friday draft", 2))
        );
        assert_eq!(
            shared_table(&Screen::Build, false, &lobby, GATEWAY),
            Some(shared("wait-1", "Friday draft", 2))
        );
        // in a running game, that game, whatever table it also waits at
        assert_eq!(
            shared_table(&seat(0, false), false, &lobby, GATEWAY),
            Some(shared("g", "Running", 4))
        );
        // a local gateway is named by its loopback origin
        assert_eq!(
            shared_table(&Screen::Table, false, &lobby, "http://127.0.0.1:28766")
                .map(|table| table.origin),
            Some("http://127.0.0.1:28766".to_owned())
        );
    }

    #[test]
    fn nothing_is_named_without_a_table_it_may_name() {
        let mut locked = listed("wait-1", "Secret", "waiting", 2, true);
        locked.locked = true;
        let locked = listing(vec![locked]);
        let open = listing(vec![listed("wait-1", "Open", "waiting", 2, true)]);
        // a table with a password, never
        assert_eq!(shared_table(&Screen::Table, false, &locked, GATEWAY), None);
        // no chair anywhere, a result, a watched game, a replay
        let nowhere = listing(vec![listed("other", "x", "waiting", 1, false)]);
        assert_eq!(shared_table(&Screen::Table, false, &nowhere, GATEWAY), None);
        assert_eq!(shared_table(&seat(0, false), true, &nowhere, GATEWAY), None);
        assert_eq!(
            shared_table(&seat(SeatHandover::WATCH, false), false, &open, GATEWAY),
            None
        );
        // the house AI in this process, and a game the listing does not hold
        assert_eq!(shared_table(&seat(0, true), false, &open, GATEWAY), None);
        assert_eq!(
            shared_table(&seat(0, false), false, &nowhere, GATEWAY),
            None
        );
        // a gateway address Central would not take: no table, whatever else
        for gateway in [
            "",
            "not a url",
            "http://gateway.example",
            "http://localhost:28766",
            "file:///tmp/x",
        ] {
            assert_eq!(
                shared_table(&Screen::Table, false, &open, gateway),
                None,
                "{gateway}"
            );
        }
        // a game id Central would not take
        let odd = listing(vec![listed("../admin", "x", "waiting", 2, true)]);
        assert_eq!(shared_table(&Screen::Table, false, &odd, GATEWAY), None);
    }

    #[test]
    fn a_name_is_named_plainly_and_within_its_bounds() {
        let name = |raw: &str| {
            shared_table(
                &Screen::Table,
                false,
                &listing(vec![listed("w", raw, "waiting", 1, true)]),
                GATEWAY,
            )
            .map(|table| table.name)
        };
        assert_eq!(name(""), Some(String::new()));
        assert_eq!(
            name(" \u{200B}Friday\u{200E} \"draft\" "),
            Some("Friday \"draft\"".to_owned())
        );
        assert_eq!(
            name(&"ü".repeat(80)),
            Some("ü".repeat(Table::MAX_NAME_CHARS))
        );
    }

    #[test]
    fn an_origin_is_the_canonical_spelling_of_the_address() {
        for (address, origin) in [
            (
                "https://gateway.baylee.app",
                Some("https://gateway.baylee.app"),
            ),
            (
                "https://Gateway.Baylee.App/",
                Some("https://gateway.baylee.app"),
            ),
            (
                "https://gateway.baylee.app:443",
                Some("https://gateway.baylee.app"),
            ),
            (
                "https://gateway.example:8443/lobby",
                Some("https://gateway.example:8443"),
            ),
            ("http://127.0.0.1:28766/", Some("http://127.0.0.1:28766")),
            ("http://[::1]:28766", Some("http://[::1]:28766")),
            ("not a url", None),
            ("data:text/plain,x", None),
        ] {
            assert_eq!(origin_of(address).as_deref(), origin, "{address}");
        }
    }
}
