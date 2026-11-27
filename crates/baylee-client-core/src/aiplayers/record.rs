//! "Gegen KIs" (`docs/proposals/ai-players.md` §"For people"): a person's
//! ranked record against AI players by kind, and the AIs they have beaten,
//! with the game each was first beaten in.
//!
//! **This device's count.** Central sends no history of who a person played,
//! only each settled game's `opponent_ai`, so the client counts the games it
//! entered and saw settled; a game played on another device is not here, and
//! the profile says so. Each game counts once however often its receipt is
//! read again ([`Record::count`]).

use super::{AiBadge, AiKind, label};
use crate::i18n::Lang;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How a settled game went from the person's seat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The person won.
    Won,
    /// The person lost.
    Lost,
    /// Neither.
    Drawn,
}

/// Wins, losses and draws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tally {
    /// Games won.
    pub wins: u32,
    /// Games lost.
    pub losses: u32,
    /// Games drawn.
    pub draws: u32,
}

impl Tally {
    /// Every game counted.
    #[must_use]
    pub const fn games(&self) -> u32 {
        self.wins
            .saturating_add(self.losses)
            .saturating_add(self.draws)
    }

    fn add(&mut self, verdict: Verdict) {
        let slot = match verdict {
            Verdict::Won => &mut self.wins,
            Verdict::Lost => &mut self.losses,
            Verdict::Drawn => &mut self.draws,
        };
        *slot = slot.saturating_add(1);
    }
}

/// One account's games against AI players on this device.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    /// By kind's stable key (`house`, `model`, `net`, `other`).
    kinds: BTreeMap<String, Tally>,
    /// Each AI beaten, by its label, with the game it was first beaten in.
    firsts: BTreeMap<String, String>,
    /// The order the AIs were first beaten in, newest last.
    beaten: Vec<String>,
    /// The last games counted, so a receipt read twice counts once.
    counted: Vec<String>,
}

/// The most AIs a record keeps as beaten, and games it remembers counting.
const KEPT: usize = 64;

fn kind_key(kind: AiKind) -> &'static str {
    kind.as_str().unwrap_or("other")
}

/// A game's id as the record keeps it.
fn game_key(id: [u8; 16]) -> String {
    uuid::Uuid::from_bytes(id).simple().to_string()
}

/// What counting a game found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counted {
    /// It was counted before.
    Again,
    /// Counted.
    Counted,
    /// Counted, and it was the person's first win against this AI.
    FirstWin,
}

impl Record {
    /// Whether no game is counted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.kinds.values().all(|t| t.games() == 0)
    }

    /// Counts the settled game `game` against `badge`'s AI.
    pub fn count(&mut self, game: [u8; 16], badge: &AiBadge, verdict: Verdict) -> Counted {
        let key = game_key(game);
        if self.counted.contains(&key) {
            return Counted::Again;
        }
        self.counted.push(key.clone());
        if self.counted.len() > KEPT {
            self.counted.remove(0);
        }
        self.kinds
            .entry(kind_key(badge.kind).to_owned())
            .or_default()
            .add(verdict);
        if verdict != Verdict::Won {
            return Counted::Counted;
        }
        let name = label(badge, Lang::En);
        if self.firsts.contains_key(&name) {
            return Counted::Counted;
        }
        self.firsts.insert(name.clone(), key);
        self.beaten.push(name);
        while self.beaten.len() > KEPT {
            let gone = self.beaten.remove(0);
            self.firsts.remove(&gone);
        }
        Counted::FirstWin
    }

    /// The record by kind, in the badge's order, kinds never met left out.
    #[must_use]
    pub fn by_kind(&self) -> Vec<(AiKind, Tally)> {
        [AiKind::House, AiKind::Net, AiKind::Model, AiKind::Other]
            .into_iter()
            .filter_map(|kind| {
                let tally = self.kinds.get(kind_key(kind)).copied()?;
                (tally.games() > 0).then_some((kind, tally))
            })
            .collect()
    }

    /// The AIs beaten, newest first.
    pub fn beaten(&self) -> impl Iterator<Item = &str> {
        self.beaten.iter().rev().map(String::as_str)
    }

    /// Whether `game` was the person's first win against `badge`'s AI.
    #[must_use]
    pub fn first_win_in(&self, game: [u8; 16], badge: &AiBadge) -> bool {
        self.firsts.get(&label(badge, Lang::En)) == Some(&game_key(game))
    }
}

/// Every account's record on this device, by its Central public id.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Records(BTreeMap<String, Record>);

impl Records {
    /// Whether no account has a record here.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// `account`'s record, if it has one here.
    #[must_use]
    pub fn of(&self, account: &str) -> Option<&Record> {
        self.0.get(account)
    }

    /// `account`'s record to count into; the device keeps at most
    /// [`super::Remembered::MAX`] accounts', the others giving way.
    pub fn of_mut(&mut self, account: &str) -> &mut Record {
        if !self.0.contains_key(account) {
            while self.0.len() >= super::Remembered::MAX {
                let Some(first) = self.0.keys().next().cloned() else {
                    break;
                };
                self.0.remove(&first);
            }
        }
        self.0.entry(account.to_owned()).or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn badge(kind: AiKind, label: &str) -> AiBadge {
        AiBadge {
            kind,
            label: label.into(),
        }
    }

    #[test]
    fn a_game_counts_once_by_kind_and_a_first_win_is_marked_once() {
        let mut record = Record::default();
        let house = badge(AiKind::House, "Haus-KI · Expert");
        assert!(record.is_empty());
        assert_eq!(
            record.count([1; 16], &house, Verdict::Lost),
            Counted::Counted
        );
        assert_eq!(record.count([1; 16], &house, Verdict::Lost), Counted::Again);
        assert_eq!(
            record.count([2; 16], &house, Verdict::Won),
            Counted::FirstWin
        );
        assert_eq!(
            record.count([3; 16], &house, Verdict::Won),
            Counted::Counted
        );
        let model = badge(AiKind::Model, "Claude Haiku · Hoch");
        assert_eq!(
            record.count([4; 16], &model, Verdict::Drawn),
            Counted::Counted
        );
        assert_eq!(
            record.by_kind(),
            vec![
                (
                    AiKind::House,
                    Tally {
                        wins: 2,
                        losses: 1,
                        draws: 0
                    }
                ),
                (
                    AiKind::Model,
                    Tally {
                        wins: 0,
                        losses: 0,
                        draws: 1
                    }
                ),
            ]
        );
        assert_eq!(record.beaten().collect::<Vec<_>>(), ["Haus-KI · Expert"]);
        assert!(record.first_win_in([2; 16], &house));
        assert!(!record.first_win_in([3; 16], &house));
        let text = serde_json::to_string(&record).unwrap();
        assert_eq!(serde_json::from_str::<Record>(&text).unwrap(), record);
    }

    #[test]
    fn the_record_and_the_book_stay_bounded() {
        let mut record = Record::default();
        for n in 0..=u8::try_from(KEPT + 4).unwrap() {
            let ai = badge(AiKind::House, &format!("Haus-KI · {n}"));
            record.count([n; 16], &ai, Verdict::Won);
        }
        assert_eq!(record.beaten.len(), KEPT);
        assert_eq!(record.firsts.len(), KEPT);
        assert_eq!(record.counted.len(), KEPT);
        let mut book = Records::default();
        for n in 0..super::super::Remembered::MAX + 3 {
            book.of_mut(&format!("account-{n:03}"));
        }
        assert_eq!(book.0.len(), super::super::Remembered::MAX);
    }
}
