//! Leads over decisions: their mean per 100 and its standard error, by
//! decision and by game.

use std::collections::BTreeMap;

/// A count as a float, for means.
pub(crate) fn real(n: u64) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Leads over decisions: their count, how many replaced the policy's
/// answer, the sum and sum of squares of the leads, and per game the
/// decisions and the sum of their leads.
#[derive(Clone, Default)]
pub(crate) struct Leads {
    pub(crate) decisions: u64,
    pub(crate) replaced: u64,
    sum: f64,
    squares: f64,
    /// Per game: (decisions, sum of leads).
    pub(crate) games: BTreeMap<u64, (u64, f64)>,
}

impl Leads {
    pub(crate) fn add(&mut self, game: u64, replaced: bool, lead: f64) {
        self.decisions += 1;
        self.replaced += u64::from(replaced);
        self.sum += lead;
        self.squares += lead * lead;
        let entry = self.games.entry(game).or_default();
        entry.0 += 1;
        entry.1 += lead;
    }

    /// The lead per 100 decisions and its standard error by decision.
    pub(crate) fn per_hundred(&self) -> (f64, f64) {
        if self.decisions == 0 {
            return (0.0, 0.0);
        }
        let n = real(self.decisions);
        let mean = self.sum / n;
        let variance = (self.squares / n - mean * mean).max(0.0);
        (100.0 * mean, 100.0 * (variance / n).sqrt())
    }

    /// The standard error per 100 decisions with each game one cluster:
    /// the ratio estimator's, `G / (G - 1) · Σ (S_g − r·n_g)² / N²`.
    pub(crate) fn per_hundred_se_by_game(&self) -> f64 {
        let clusters = real(self.games.len() as u64);
        if self.decisions == 0 || clusters < 2.0 {
            return 0.0;
        }
        let n = real(self.decisions);
        let ratio = self.sum / n;
        let spread: f64 = self
            .games
            .values()
            .map(|&(decisions, sum)| (sum - ratio * real(decisions)).powi(2))
            .sum();
        100.0 * (clusters / (clusters - 1.0) * spread / (n * n)).sqrt()
    }

    pub(crate) fn report(&self) -> serde_json::Value {
        let (lead, se) = self.per_hundred();
        serde_json::json!({
            "decisions": self.decisions,
            "games": self.games.len(),
            "replaced": self.replaced,
            "replaced_share": if self.decisions == 0 { 0.0 } else {
                real(self.replaced) / real(self.decisions)
            },
            "lead_per_100": lead,
            "lead_per_100_se": se,
            "lead_per_100_se_by_game": self.per_hundred_se_by_game(),
        })
    }
}
