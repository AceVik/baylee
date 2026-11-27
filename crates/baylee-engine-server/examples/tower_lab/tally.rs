//! Counting duels per build and the interval each count supports.
use crate::duel::{Played, Result};
use serde::Serialize;

/// One build's duels, from its own side of the table.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Tally {
    /// Won through the rules.
    pub wins: u32,
    /// Lost through the rules.
    pub losses: u32,
    /// Rules draws.
    pub draws: u32,
    /// Ended by a bound; not a result and not in the score.
    pub unfinished: u32,
    /// Refused by the executor; not a duel at all.
    pub refused: u32,
    /// Sum of rules turns over finished duels.
    pub turns: u64,
    /// Wall time over every duel, in milliseconds.
    pub millis: u64,
}

impl Tally {
    /// Count one duel as seen by `first` (seat 0) or the other build.
    pub fn add(&mut self, played: &anyhow::Result<Played>, as_first: bool) {
        let Ok(played) = played else {
            self.refused += 1;
            return;
        };
        self.millis += played.millis;
        match (played.result, as_first) {
            (Result::First, true) | (Result::Second, false) => self.wins += 1,
            (Result::Second, true) | (Result::First, false) => self.losses += 1,
            (Result::Draw, _) => self.draws += 1,
            (Result::Unfinished, _) => {
                self.unfinished += 1;
                return;
            }
        }
        self.turns += u64::from(played.turns);
    }

    /// Merge another count of the same build.
    pub fn merge(&mut self, other: &Self) {
        self.wins += other.wins;
        self.losses += other.losses;
        self.draws += other.draws;
        self.unfinished += other.unfinished;
        self.refused += other.refused;
        self.turns += other.turns;
        self.millis += other.millis;
    }

    /// Duels that ended through the rules.
    #[must_use]
    pub const fn decided(&self) -> u32 {
        self.wins + self.losses + self.draws
    }

    /// Wins plus half the draws over decided duels.
    #[must_use]
    pub fn score(&self) -> Option<f64> {
        let n = self.decided();
        (n > 0).then(|| (f64::from(self.wins) + f64::from(self.draws) / 2.0) / f64::from(n))
    }

    /// The 95 % Wilson score interval of [`Self::score`].
    #[must_use]
    pub fn interval(&self) -> Option<(f64, f64)> {
        let n = f64::from(self.decided());
        let p = self.score()?;
        let z = 1.96_f64;
        let centre = (p + z * z / (2.0 * n)) / (1.0 + z * z / n);
        let half = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt() / (1.0 + z * z / n);
        Some((centre - half, centre + half))
    }

    /// Report form: counts, score and interval.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // sums of turns and milliseconds, far below 2^52
    pub fn report(&self) -> serde_json::Value {
        let decided = self.decided();
        serde_json::json!({
            "wins": self.wins,
            "losses": self.losses,
            "draws": self.draws,
            "unfinished": self.unfinished,
            "refused": self.refused,
            "score": self.score().map(round),
            "interval": self.interval().map(|(low, high)| [round(low), round(high)]),
            "mean_turns": (decided > 0).then(|| round(self.turns as f64 / f64::from(decided))),
            "mean_seconds": (decided + self.unfinished > 0)
                .then(|| round(self.millis as f64 / 1000.0 / f64::from(decided + self.unfinished))),
        })
    }
}

/// Three decimals: enough for a rate measured over at most a few thousand duels.
#[must_use]
pub fn round(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}
