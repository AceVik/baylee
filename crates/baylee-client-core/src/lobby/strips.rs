//! What the shell's header and strips say (the shell design, §2.1, §2.5):
//! the seated strip, the gateway pill's counts, the account pill's handle.
//!
//! Decided here, drawn by the client: the strip is a fact about the lobby's
//! state (which table holds this player's chair, and whether it is being
//! played), the counts are the gateway's `GET /lobby/stats` or, until it has
//! answered, the two numbers the table listing knows, and the handle is
//! `GET /me`'s — never the username, which is private.

use serde::Deserialize;

use super::{GameSummary, Lobby, Screen};
use crate::i18n::{Lang, Phrase};

/// `GET /lobby/stats` (WG-0): three numbers, read from the gateway's memory
/// at the moment of asking.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct LobbyStats {
    /// Distinct accounts with a lobby socket open or a chair in a running
    /// game.
    pub players_online: u32,
    /// Tables waiting for players.
    pub tables_waiting: u32,
    /// Games being played.
    pub games_running: u32,
}

/// `GET /me`: who the account pill names. The username is the owner's to
/// see and nobody's to be shown by, so the pill reads the handle.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Me {
    /// `Name#tag`, what every other player sees.
    #[serde(default)]
    pub handle: String,
    /// A guest is labelled as one.
    #[serde(default)]
    pub guest: bool,
}

/// The seated strip (§2.1, M-7): the player holds a chair somewhere, and
/// every screen but the room itself says so, with the way back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Strip {
    /// A chair at a waiting table: Return and Leave.
    Seated {
        /// The table's index in the listing.
        index: usize,
        /// What the table is called.
        table: String,
        /// Chairs taken.
        seated: usize,
        /// Chairs there are.
        chairs: usize,
        /// Chairs ready.
        ready: usize,
    },
    /// A seat in a game being played: Return alone (leaving a running game
    /// from the lobby would be a concession).
    Playing {
        /// The game's id.
        game_id: String,
        /// What the table is called.
        table: String,
    },
}

impl Strip {
    /// The strip's sentence.
    #[must_use]
    pub fn sentence(&self, lang: Lang) -> String {
        match self {
            Self::Seated {
                table,
                seated,
                chairs,
                ready,
                ..
            } => Phrase::ShellSeatedStrip.fill(
                lang,
                &[
                    table,
                    &seated.to_string(),
                    &chairs.to_string(),
                    &ready.to_string(),
                ],
            ),
            Self::Playing { table, .. } => Phrase::ShellPlayingStrip.fill(lang, &[table]),
        }
    }

    /// Whether the strip offers Leave: only at a waiting table.
    #[must_use]
    pub const fn can_leave(&self) -> bool {
        matches!(self, Self::Seated { .. })
    }
}

/// What a table is called on the strip: its name, else its host's.
#[must_use]
pub fn table_name(game: &GameSummary) -> String {
    if !game.name.trim().is_empty() {
        return game.name.trim().to_string();
    }
    game.host
        .clone()
        .unwrap_or_else(|| game.id.chars().take(6).collect())
}

/// The strip the lobby's state calls for, if any.
///
/// A chair at a waiting table (the one this client waits at, or any listed
/// table that seats this player and is not over), or a seat in a game being
/// played that this client is not at. `None` once the player is at the
/// table, and on the front door, which has no account.
#[must_use]
pub fn strip(lobby: &Lobby) -> Option<Strip> {
    if matches!(lobby.screen(), Screen::SignIn { .. } | Screen::Seated(_)) {
        return None;
    }
    let games = lobby.games();
    let waiting_at = lobby.awaiting().map(|h| h.game_id.as_str());
    let waiting = games.iter().enumerate().find(|(_, g)| {
        g.state == "waiting" && (Some(g.id.as_str()) == waiting_at || g.mine().is_some())
    });
    if let Some((index, game)) = waiting {
        let taken = game.seats.iter().filter(|s| s.taken).count();
        return Some(Strip::Seated {
            index,
            table: table_name(game),
            seated: taken,
            chairs: game.seats.len(),
            ready: game.seats.iter().filter(|s| s.taken && s.ready).count(),
        });
    }
    games
        .iter()
        .find(|g| g.state == "playing" && g.mine().is_some())
        .map(|g| Strip::Playing {
            game_id: g.id.clone(),
            table: table_name(g),
        })
}

/// Whether the screen on show is the room the strip would return to, where
/// the strip is never drawn (the room has its own Leave).
#[must_use]
pub fn in_the_room(lobby: &Lobby, settings_open: bool) -> bool {
    if settings_open || lobby.library().page.is_some() {
        return false;
    }
    let Some(handover) = lobby.awaiting() else {
        return false;
    };
    matches!(lobby.screen(), Screen::Table)
        && lobby
            .games()
            .iter()
            .any(|g| g.id == handover.game_id && g.state == "waiting")
}

/// The gateway pill's two counts: tables waiting and players online, from
/// `GET /lobby/stats` once it has answered; before that the tables the
/// listing knows and no player count (the listing cannot know it).
#[must_use]
pub fn pill_counts(stats: Option<&LobbyStats>, lobby: &Lobby) -> (u32, Option<u32>) {
    if let Some(stats) = stats {
        return (stats.tables_waiting, Some(stats.players_online));
    }
    let waiting = lobby
        .games()
        .iter()
        .filter(|g| g.state == "waiting")
        .count();
    (u32::try_from(waiting).unwrap_or(u32::MAX), None)
}

/// The gateway pill's words after its name (Wide and Vast; narrower
/// classes show the dot alone).
#[must_use]
pub fn pill_words(stats: Option<&LobbyStats>, lobby: &Lobby, lang: Lang) -> String {
    match pill_counts(stats, lobby) {
        (tables, Some(online)) => {
            Phrase::ShellTablesOnline.fill(lang, &[&tables.to_string(), &online.to_string()])
        }
        (tables, None) => Phrase::ShellTablesWaiting.fill(lang, &[&tables.to_string()]),
    }
}

/// Something that happened which the player may have missed (§2.1 Bell):
/// only the missable — a chair taken or given up at their table, the table
/// filled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BellItem {
    /// Somebody sat down at the player's table.
    Joined(String),
    /// Somebody left it.
    Left(String),
    /// Every chair at it is taken.
    Filled,
}

impl BellItem {
    /// The item as a sentence.
    #[must_use]
    pub fn sentence(&self, lang: Lang) -> String {
        match self {
            Self::Joined(who) => Phrase::ShellBellJoined.fill(lang, &[who]),
            Self::Left(who) => Phrase::ShellBellLeft.fill(lang, &[who]),
            Self::Filled => Phrase::ShellBellFilled.text(lang).to_string(),
        }
    }
}

/// The bell: what happened, newest last, and how much of it is unread.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bell {
    items: Vec<BellItem>,
    unread: usize,
    /// How many times it has rung, ever: what a toast lane counts against.
    rung: u64,
}

impl Bell {
    /// How many items the bell keeps.
    pub const KEPT: usize = 20;

    /// Rings: one more item, unread.
    pub fn ring(&mut self, item: BellItem) {
        if self.items.len() == Self::KEPT {
            self.items.remove(0);
        }
        self.items.push(item);
        self.unread = (self.unread + 1).min(Self::KEPT);
        self.rung += 1;
    }

    /// How many times it has rung since the lobby began.
    #[must_use]
    pub const fn rung(&self) -> u64 {
        self.rung
    }

    /// The newest `n` items, oldest of them first.
    pub fn latest(&self, n: usize) -> impl Iterator<Item = &BellItem> {
        self.items[self.items.len().saturating_sub(n)..].iter()
    }

    /// Items not yet seen.
    #[must_use]
    pub const fn unread(&self) -> usize {
        self.unread
    }

    /// Everything, as seen.
    pub fn read_all(&mut self) {
        self.unread = 0;
    }

    /// How many items there have been (kept).
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether it has never rung.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The items as sentences, newest first.
    #[must_use]
    pub fn lines(&self, lang: Lang) -> Vec<String> {
        self.items.iter().rev().map(|i| i.sentence(lang)).collect()
    }

    /// The newest item.
    #[must_use]
    pub fn newest(&self) -> Option<&BellItem> {
        self.items.last()
    }
}

/// What changed at the player's waiting table between two listings: who sat
/// down, who left, whether it filled. Nothing for a table that is not the
/// same table, or for the player's own chair.
#[must_use]
pub fn table_news(before: &GameSummary, after: &GameSummary) -> Vec<BellItem> {
    if before.id != after.id || after.state != "waiting" {
        return Vec::new();
    }
    let others = |g: &GameSummary| -> Vec<String> {
        g.seats
            .iter()
            .filter(|s| s.taken && !s.you)
            .filter_map(|s| s.player.clone())
            .collect()
    };
    let (was, now) = (others(before), others(after));
    let mut news: Vec<BellItem> = now
        .iter()
        .filter(|p| !was.contains(p))
        .map(|p| BellItem::Joined(p.clone()))
        .collect();
    news.extend(
        was.iter()
            .filter(|p| !now.contains(p))
            .map(|p| BellItem::Left(p.clone())),
    );
    let full = |g: &GameSummary| !g.seats.is_empty() && g.seats.iter().all(|s| s.taken);
    if full(after) && !full(before) {
        news.push(BellItem::Filled);
    }
    news
}

/// The player's own waiting table in a listing, if any.
#[must_use]
pub fn my_waiting_table(lobby: &Lobby) -> Option<&GameSummary> {
    lobby
        .games()
        .iter()
        .find(|g| g.state == "waiting" && g.mine().is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lobby::{GameListing, GameSeat, LobbyEvent, SeatHandover};

    fn seat(n: u32, taken: bool, you: bool, ready: bool) -> GameSeat {
        GameSeat {
            seat: n,
            taken,
            you,
            ready,
            ..GameSeat::default()
        }
    }

    fn game(id: &str, name: &str, state: &str, seats: Vec<GameSeat>) -> GameSummary {
        GameSummary {
            id: id.to_string(),
            name: name.to_string(),
            host: Some("Maik#0012".to_string()),
            state: state.to_string(),
            seats,
            ..GameSummary::default()
        }
    }

    fn signed_in(games: Vec<GameSummary>) -> Lobby {
        let mut lobby = Lobby::default();
        lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
            username: None,
        });
        lobby.apply(LobbyEvent::Games(GameListing {
            total: games.len(),
            games,
            ..GameListing::default()
        }));
        lobby
    }

    #[test]
    fn a_chair_at_a_waiting_table_is_a_seated_strip_with_its_counts() {
        let lobby = signed_in(vec![
            game("a", "", "waiting", vec![seat(0, true, false, true)]),
            game(
                "b",
                "Thursday pod",
                "waiting",
                vec![
                    seat(0, true, true, true),
                    seat(1, true, false, false),
                    seat(2, true, false, false),
                    seat(3, false, false, false),
                ],
            ),
        ]);
        let strip = strip(&lobby).expect("seated");
        assert_eq!(
            strip,
            Strip::Seated {
                index: 1,
                table: "Thursday pod".to_string(),
                seated: 3,
                chairs: 4,
                ready: 1
            }
        );
        assert!(strip.can_leave());
        assert_eq!(
            strip.sentence(Lang::En),
            "Seated at \u{201c}Thursday pod\u{201d} · 3 of 4 seated · 1 ready"
        );
        assert!(strip.sentence(Lang::De).contains("Thursday pod"));
    }

    #[test]
    fn a_game_being_played_offers_return_alone() {
        let lobby = signed_in(vec![game(
            "g",
            "",
            "playing",
            vec![seat(0, true, true, true), seat(1, true, false, true)],
        )]);
        let strip = strip(&lobby).expect("playing");
        assert!(matches!(&strip, Strip::Playing { table, .. } if table == "Maik#0012"));
        assert!(!strip.can_leave());
    }

    #[test]
    fn nobody_seated_no_strip_and_never_on_the_front_door() {
        let lobby = signed_in(vec![game(
            "a",
            "x",
            "waiting",
            vec![seat(0, true, false, false)],
        )]);
        assert_eq!(strip(&lobby), None);
        assert_eq!(strip(&Lobby::default()), None);
    }

    #[test]
    fn the_room_is_where_the_strip_is_not() {
        let mut lobby = signed_in(vec![game(
            "r",
            "pod",
            "waiting",
            vec![seat(0, true, true, false), seat(1, false, false, false)],
        )]);
        assert!(!in_the_room(&lobby, false));
        lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "r".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
            local: false,
        }));
        assert!(strip(&lobby).is_some());
        assert!(in_the_room(&lobby, false));
        assert!(!in_the_room(&lobby, true), "Settings stands over the room");
    }

    #[test]
    fn the_pill_counts_the_gateway_s_numbers_once_it_has_answered() {
        let lobby = signed_in(vec![
            game("a", "", "waiting", vec![]),
            game("b", "", "playing", vec![]),
        ]);
        assert_eq!(pill_counts(None, &lobby), (1, None));
        assert_eq!(pill_words(None, &lobby, Lang::En), "1 waiting");
        let stats = LobbyStats {
            players_online: 12,
            tables_waiting: 3,
            games_running: 4,
        };
        assert_eq!(pill_counts(Some(&stats), &lobby), (3, Some(12)));
        assert_eq!(
            pill_words(Some(&stats), &lobby, Lang::En),
            "3 tables · 12 online"
        );
        let read: LobbyStats =
            serde_json::from_str(r#"{"players_online":12,"tables_waiting":3,"games_running":4}"#)
                .expect("the wire shape");
        assert_eq!(read, stats);
        let me: Me = serde_json::from_str(
            r#"{"id":"x","email":null,"username":"vik","guest":false,"display_name":"Vik","tag":"0007","handle":"Vik#0007"}"#,
        )
        .expect("the wire shape");
        assert_eq!(me.handle, "Vik#0007");
    }

    #[test]
    fn the_bell_rings_for_who_came_and_went_and_a_full_table() {
        let seated = |who: Option<&str>, you: bool| GameSeat {
            taken: who.is_some(),
            you,
            player: who.map(str::to_string),
            ..GameSeat::default()
        };
        let before = game(
            "t",
            "pod",
            "waiting",
            vec![
                seated(Some("me#1"), true),
                seated(Some("Ole#9"), false),
                seated(None, false),
            ],
        );
        let after = game(
            "t",
            "pod",
            "waiting",
            vec![
                seated(Some("me#1"), true),
                seated(None, false),
                seated(Some("Lina#3"), false),
            ],
        );
        assert_eq!(
            table_news(&before, &after),
            [
                BellItem::Joined("Lina#3".to_string()),
                BellItem::Left("Ole#9".to_string())
            ]
        );
        let full = game(
            "t",
            "pod",
            "waiting",
            vec![
                seated(Some("me#1"), true),
                seated(Some("Ole#9"), false),
                seated(Some("Lina#3"), false),
            ],
        );
        assert_eq!(
            table_news(&after, &full),
            [BellItem::Joined("Ole#9".to_string()), BellItem::Filled]
        );
        assert!(table_news(&full, &full).is_empty());
        let other = game("u", "pod", "waiting", vec![]);
        assert!(
            table_news(&full, &other).is_empty(),
            "a different table is no news"
        );
        let mut bell = Bell::default();
        for item in table_news(&before, &after) {
            bell.ring(item);
        }
        assert_eq!(bell.unread(), 2);
        assert_eq!(bell.lines(Lang::En)[0], "Ole#9 left your table");
        bell.read_all();
        assert_eq!(bell.unread(), 0);
        assert_eq!(bell.len(), 2);
    }

    /// §2.5 (S4-12): the agent or engine gone while in a room — Start is
    /// answered `503` — leaves the gateway's sentence, the seat kept, and
    /// Start offered again.
    #[test]
    fn a_start_the_gateway_cannot_run_keeps_the_seat_and_can_be_retried() {
        let mut lobby = signed_in(vec![game(
            "r",
            "pod",
            "waiting",
            vec![seat(0, true, true, true), seat(1, true, false, true)],
        )]);
        lobby.apply(LobbyEvent::Seated(SeatHandover {
            game_id: "r".to_string(),
            seat: 0,
            seat_token: "st".to_string(),
            local: false,
        }));
        // The listing the seat asked for answers, and nothing is in flight.
        let listing = lobby.games().to_vec();
        lobby.apply(LobbyEvent::Games(GameListing {
            total: listing.len(),
            games: listing,
            ..GameListing::default()
        }));
        assert!(!lobby.busy());
        assert!(lobby.start_room("r").is_some());
        let said = "no agent is connected to start the game";
        lobby.apply(LobbyEvent::Failed(said.to_string()));
        assert_eq!(lobby.status(), said, "the gateway's words, verbatim");
        assert!(lobby.awaiting().is_some(), "the seat is kept");
        assert!(strip(&lobby).is_some());
        assert!(lobby.start_room("r").is_some(), "Start retries");
    }
}
