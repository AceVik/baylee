//! The reporting seat's own game log, with nobody's name in it.
//!
//! The log a seat holds is the one it was told (`gamelog::LogBook`): what
//! that seat saw, and nothing hidden from it. What it does carry is other
//! players' names, which the log writes into its lines out of the roster
//! (`i18n::seat_name`). So the lines are written here against a copy of the
//! roster whose names are already placeholders — the reporter "You", every
//! other seat "Player A", "Player B", … in seat order — and no line is ever
//! written with a real name to be scrubbed afterwards.
//!
//! The lines are written in English whatever the interface speaks: they are
//! read by whoever reads reports, and one language there is one search.

use serde::Serialize;

use baylee_core::ids::PlayerId;
use baylee_view::GameStatic;

use crate::gamelog::{CardTextLookup, LogBook, Wording};
use crate::i18n::Lang;

/// One seat of the game, as a report names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RosterSeat {
    /// The seat number.
    pub seat: u8,
    /// "You" or "Player A", "Player B", …: never the seat's own name.
    pub name: String,
    /// Whether the house AI plays it.
    pub is_ai: bool,
    /// Its team, where the format has them.
    pub team: Option<u8>,
}

/// One line of the log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LogRow {
    /// The turn it happened in.
    pub turn: u32,
    /// The host's time, milliseconds since the Unix epoch (0: undated).
    pub at: u64,
    /// The line, in English, with placeholders for names.
    pub text: String,
}

/// The seat's log as a report carries it ([`super::Category::Log`]).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SeatLog {
    /// The reporting seat.
    pub seat: u8,
    /// Who sat where, under placeholder names.
    pub roster: Vec<RosterSeat>,
    /// The lines, oldest first.
    pub lines: Vec<LogRow>,
}

/// The placeholder for the `nth` other seat: "Player A", "Player B", …
fn placeholder(nth: usize) -> String {
    let letter = u8::try_from(nth % 26).map_or('?', |n| char::from(b'A' + n));
    format!("Player {letter}")
}

/// `book` as `seat` reads it, written for a report.
#[must_use]
pub fn seat_log(
    book: &LogBook,
    statics: Option<&GameStatic>,
    seat: PlayerId,
    texts: &dyn CardTextLookup,
) -> SeatLog {
    let redacted = statics.map(|statics| {
        let mut redacted = statics.clone();
        let mut others = 0;
        for identity in &mut redacted.seats {
            identity.display_name = if identity.player == seat {
                "You".to_string()
            } else {
                others += 1;
                placeholder(others - 1)
            };
        }
        redacted
    });
    let wording = Wording {
        lang: Lang::En,
        seat,
        statics: redacted.as_ref(),
        texts,
    };
    SeatLog {
        seat: seat.get(),
        roster: redacted.as_ref().map_or_else(Vec::new, |r| {
            r.seats
                .iter()
                .map(|identity| RosterSeat {
                    seat: identity.player.get(),
                    name: identity.display_name.clone(),
                    is_ai: identity.is_ai,
                    team: identity.team,
                })
                .collect()
        }),
        lines: book
            .lines(&wording)
            .into_iter()
            .map(|line| LogRow {
                turn: line.turn,
                at: line.at,
                text: line.plain(Lang::En),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bugreport::{Build, Consent, Gathered, Kind};
    use crate::card_face::CardText;
    use crate::test_support::{ViewBuilder, statics};
    use baylee_core::ids::CardIndex;
    use baylee_view::{LogEntry, LogEvent, LogTail, SeatIdentity};

    const NAMES: [&str; 3] = ["Adelheid#0a1b", "Bartholomew#77ff", "Cunegonde#1234"];

    fn no_text(_: CardIndex, _: u8) -> Option<CardText> {
        None
    }

    fn roster() -> GameStatic {
        let mut roster = statics(0);
        roster.seats = NAMES
            .iter()
            .zip(0..)
            .map(|(name, seat)| SeatIdentity {
                player: PlayerId::new(seat),
                display_name: (*name).to_string(),
                is_ai: seat == 2,
                away: false,
                team: None,
            })
            .collect();
        roster
    }

    /// Each seat's turn beginning, then each seat losing: lines that name
    /// every seat, the reporter's included.
    fn book() -> LogBook {
        let entries: Vec<LogEntry> = (0..3)
            .map(|seat| LogEntry {
                at: 1_700_000_000_000,
                turn: u32::from(seat) + 1,
                repeat: 1,
                event: LogEvent::TurnStarted {
                    active: PlayerId::new(seat),
                },
            })
            .collect();
        let mut book = LogBook::new();
        book.append(
            &LogTail {
                from: 0,
                entries: entries.clone(),
            },
            &ViewBuilder::new(3).build(),
        );
        assert_eq!(book.entries().len(), entries.len());
        book
    }

    /// The reporter reads seat 1's log: the others become Player A and B in
    /// seat order, the reporter is "You", and no real name is in the lines
    /// or the roster.
    #[test]
    fn other_players_are_placeholders_in_every_line() {
        let roster = roster();
        let log = seat_log(&book(), Some(&roster), PlayerId::new(1), &no_text);
        let names: Vec<_> = log.roster.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Player A", "You", "Player B"]);
        assert!(log.roster[2].is_ai);
        assert_eq!(log.lines.len(), 3);
        let all = log
            .lines
            .iter()
            .map(|row| row.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            all.contains("Player A") && all.contains("Player B"),
            "{all}"
        );
        for name in NAMES {
            let bare = name.split('#').next().expect("a name");
            assert!(!all.contains(bare), "{bare} is in the log:\n{all}");
        }
    }

    /// And in the sealed bytes, which is what counts: the real names appear
    /// nowhere in a report carrying the log, and the placeholders do.
    #[test]
    fn no_real_name_reaches_the_sealed_report() {
        let roster = roster();
        let gathered = Gathered {
            build: Build::default(),
            log: Some(seat_log(&book(), Some(&roster), PlayerId::new(0), &no_text)),
            ..Gathered::default()
        };
        let consent = Consent {
            log: true,
            ..Consent::default()
        };
        let (json, _) = gathered
            .submission(Kind::Bug, "x", &consent)
            .sealed(&[])
            .expect("sealed");
        for name in NAMES {
            let bare = name.split('#').next().expect("a name");
            assert!(!json.contains(bare), "{bare} reached the report");
        }
        assert!(json.contains("Player A") && json.contains("Player B"));
    }

    /// Without a roster the log numbers the seats, which names nobody either.
    #[test]
    fn without_a_roster_the_seats_are_numbered() {
        let log = seat_log(&book(), None, PlayerId::new(0), &no_text);
        assert!(log.roster.is_empty());
        assert_eq!(log.lines.len(), 3);
    }
}
