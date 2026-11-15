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
    /// Cards drawn initially and on every mulligan (1–9; admitted native packs: 1–32).
    pub cards: u8,
    /// Base maximum hand size for cleanup (1–9; admitted native packs: 1–32).
    pub maximum_hand: u8,
    /// Redraws before the London mulligan costs cards (0–7; admitted native packs: 0–32).
    pub free_mulligans: u8,
}

impl SeatOpening {
    fn valid(self, seats: usize, native: bool) -> bool {
        // This is only a structural bound. The selected compiled lookup must
        // still admit the exact pack and derive every resource from its IDs.
        let (hand, mulligans) = if native { (32, 32) } else { (9, 7) };
        usize::from(self.seat) < seats
            && (1..=hand).contains(&self.cards)
            && (1..=hand).contains(&self.maximum_hand)
            && self.free_mulligans <= mulligans
    }
}

pub(super) fn valid_list(rules: &[SeatOpening], seats: usize, native: bool) -> bool {
    rules.len() <= seats
        && rules.iter().enumerate().all(|(i, rule)| {
            rule.valid(seats, native)
                && !rules[..i].iter().any(|previous| previous.seat == rule.seat)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_bounds_do_not_relax_regular_openings_or_duplicate_seats() {
        let regular = SeatOpening {
            seat: 0,
            cards: 9,
            maximum_hand: 9,
            free_mulligans: 7,
        };
        assert!(valid_list(&[regular], 2, false));
        for large in [
            SeatOpening {
                cards: 10,
                ..regular
            },
            SeatOpening {
                maximum_hand: 10,
                ..regular
            },
            SeatOpening {
                free_mulligans: 8,
                ..regular
            },
            SeatOpening {
                cards: 32,
                maximum_hand: 32,
                free_mulligans: 32,
                ..regular
            },
        ] {
            assert!(!valid_list(&[large], 2, false));
            assert!(valid_list(&[large], 2, true));
        }
        for invalid in [
            SeatOpening {
                cards: 33,
                ..regular
            },
            SeatOpening {
                maximum_hand: 33,
                ..regular
            },
            SeatOpening {
                free_mulligans: 33,
                ..regular
            },
            SeatOpening {
                cards: 0,
                ..regular
            },
            SeatOpening {
                maximum_hand: 0,
                ..regular
            },
            SeatOpening { seat: 2, ..regular },
        ] {
            assert!(!valid_list(&[invalid], 2, true));
        }
        assert!(!valid_list(&[regular, regular], 2, true));
    }
}
