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
//! A language-model seat's report (`baylee_seat::reports`) carries the same
//! log, written the same way, with the placeholders its model knew the
//! others by, "P1", "P2", … ([`Placeholders::Seats`]): its reports name
//! players that way, and the log they point into should too.
//!
//! The lines are written in English whatever the interface speaks: they are
//! read by whoever reads reports, and one language there is one search.

use serde::Serialize;

use baylee_core::ids::PlayerId;
use baylee_view::GameStatic;

use crate::gamelog::{CardTextLookup, LogBook, Wording};
use crate::i18n::Lang;

/// What a report's log calls the seats other than the reporter's, who is
/// "You" either way. Neither is anybody's name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Placeholders {
    /// "Player A", "Player B", … in seat order: a person's report.
    #[default]
    Letters,
    /// "P1", "P2", … by seat number, from one: a language-model seat's
    /// report, whose model was told every seat that way and wrote its
    /// reports in those words.
    Seats,
}

impl Placeholders {
    /// The name of `seat`, the `nth` seat other than the reporter's.
    fn name(self, nth: usize, seat: PlayerId) -> String {
        match self {
            Self::Letters => placeholder(nth),
            Self::Seats => format!("P{}", u16::from(seat.get()) + 1),
        }
    }
}

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

impl SeatLog {
    /// Leaves out the oldest lines until the log, serialised, weighs at
    /// most `bytes`, and says how many it left out. The newest lines are
    /// the ones kept: a report is about the end of what it was told. A log
    /// whose roster alone is over `bytes` keeps no line.
    pub fn fit(&mut self, bytes: usize) -> usize {
        fn weight<T: Serialize + ?Sized>(value: &T) -> usize {
            serde_json::to_vec(value).map_or(usize::MAX, |json| json.len())
        }
        let whole = weight(self);
        if whole <= bytes {
            return 0;
        }
        let rows: Vec<usize> = self.lines.iter().map(weight).collect();
        // The log with no line at all: the whole, less every row and the
        // commas between them.
        let commas = rows.len().saturating_sub(1);
        let lines = rows
            .iter()
            .fold(commas, |sum, row| sum.saturating_add(*row));
        let mut used = whole.saturating_sub(lines);
        let mut kept = 0;
        for row in rows.iter().rev() {
            let next = used
                .saturating_add(*row)
                .saturating_add(usize::from(kept > 0));
            if next > bytes {
                break;
            }
            used = next;
            kept += 1;
        }
        let left_out = self.lines.len() - kept;
        self.lines.drain(..left_out);
        left_out
    }
}

/// The placeholder for the `nth` other seat: "Player A", "Player B", …
fn placeholder(nth: usize) -> String {
    let letter = u8::try_from(nth % 26).map_or('?', |n| char::from(b'A' + n));
    format!("Player {letter}")
}

/// `book` as `seat` reads it, written for a person's report: the others
/// are [`Placeholders::Letters`].
#[must_use]
pub fn seat_log(
    book: &LogBook,
    statics: Option<&GameStatic>,
    seat: PlayerId,
    texts: &dyn CardTextLookup,
) -> SeatLog {
    seat_log_with(book, statics, seat, texts, Placeholders::Letters)
}

/// `book` as `seat` reads it, written for a report, the other seats
/// named by `placeholders`.
#[must_use]
pub fn seat_log_with(
    book: &LogBook,
    statics: Option<&GameStatic>,
    seat: PlayerId,
    texts: &dyn CardTextLookup,
    placeholders: Placeholders,
) -> SeatLog {
    let redacted = statics.map(|statics| {
        let mut redacted = statics.clone();
        let mut others = 0;
        for identity in &mut redacted.seats {
            identity.display_name = if identity.player == seat {
                "You".to_string()
            } else {
                others += 1;
                placeholders.name(others - 1, identity.player)
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
                phase: None,
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

    /// A language-model seat's log names the others as its model knew
    /// them, by seat number from one, and the reporter "You"; still no real
    /// name anywhere, and the same lines as a person's log of the book.
    #[test]
    fn a_model_s_log_names_the_others_by_seat_number() {
        let roster = roster();
        let log = seat_log_with(
            &book(),
            Some(&roster),
            PlayerId::new(1),
            &no_text,
            Placeholders::Seats,
        );
        let names: Vec<_> = log.roster.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["P1", "You", "P3"]);
        let json = serde_json::to_string(&log).expect("json");
        for name in NAMES {
            let bare = name.split('#').next().expect("a name");
            assert!(!json.contains(bare), "{bare} is in the log: {json}");
        }
        let all: Vec<&str> = log.lines.iter().map(|row| row.text.as_str()).collect();
        assert!(all.iter().any(|line| line.contains("P1")), "{all:?}");
        assert!(all.iter().any(|line| line.contains("P3")), "{all:?}");
        let person = seat_log(&book(), Some(&roster), PlayerId::new(1), &no_text);
        assert_eq!(log.lines.len(), person.lines.len());
        assert_eq!(log.seat, person.seat);
    }

    /// A log over its bound loses its oldest lines, the newest kept, and
    /// weighs no more than the bound after; one within it is left whole.
    #[test]
    fn a_log_is_fitted_from_its_start() {
        let mut log = SeatLog {
            seat: 0,
            roster: vec![RosterSeat {
                seat: 0,
                name: "You".into(),
                is_ai: false,
                team: None,
            }],
            lines: (0..200)
                .map(|n| LogRow {
                    turn: n,
                    at: 1_700_000_000_000,
                    text: format!("line {n:03} {}", "ä".repeat(20)),
                })
                .collect(),
        };
        let whole = serde_json::to_vec(&log).expect("json").len();
        assert_eq!(log.clone().fit(whole), 0, "a log within its bound is whole");
        let bound = whole / 3;
        let left_out = log.fit(bound);
        let weight = serde_json::to_vec(&log).expect("json").len();
        assert!(weight <= bound, "{weight} > {bound}");
        assert_eq!(log.lines.len() + left_out, 200);
        assert_eq!(log.lines.last().map(|row| row.turn), Some(199));
        assert_eq!(
            log.lines.first().map(|row| row.turn),
            Some(200 - log.lines.len() as u32)
        );
        // One more line would not have fitted: as many as fit are kept.
        let row = serde_json::to_vec(&log.lines[0]).expect("json").len();
        assert!(weight + row + 1 > bound, "a line too many was left out");
        // A bound under the roster keeps no line.
        let mut tiny = log.clone();
        assert_eq!(tiny.fit(10), log.lines.len());
        assert!(tiny.lines.is_empty());
    }
}
