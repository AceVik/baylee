//! Bounded per-seat opening rules for custom games, including Tower builds.

use serde::{Deserialize, Serialize};

/// A seat's opening draw, London mulligan allowance and base hand limit.
///
/// The explicit `SeatSpec::starting_hand` still takes precedence for the first
/// hand; redraws use `cards`. Card effects can change `maximum_hand` in play.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct SeatOpening {
    /// Zero-based seat in the preset.
    pub seat: u8,
    /// Cards drawn initially and on every mulligan (1–9).
    pub cards: u8,
    /// Base maximum hand size for cleanup (1–9).
    pub maximum_hand: u8,
    /// Redraws before the London mulligan costs cards (0–7).
    pub free_mulligans: u8,
}

impl SeatOpening {
    pub(super) fn valid(self, seats: usize) -> bool {
        usize::from(self.seat) < seats
            && (1..=9).contains(&self.cards)
            && (1..=9).contains(&self.maximum_hand)
            && self.free_mulligans <= 7
    }
}

pub(super) fn valid_list(rules: &[SeatOpening], seats: usize) -> bool {
    rules.len() <= seats
        && rules.iter().enumerate().all(|(i, rule)| {
            rule.valid(seats) && !rules[..i].iter().any(|previous| previous.seat == rule.seat)
        })
}
