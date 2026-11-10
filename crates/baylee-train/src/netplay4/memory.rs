//! `eval4 --memory-reset`: a measurement of what the net's recurrent memory H
//! buys. With it the seat under test forgets H at a fixed point and answers
//! from its current view alone; the explicit observation ledger
//! (`SeatKnowledge4`: own-deck inventory, revealed cards, public history
//! counters) is part of the encoded view and is kept, since it is not the
//! learned memory under test.
//!
//! The reset happens before the observation update (K=0) of a view, never
//! between that update and the decision readout: a seat that forgets reads the
//! view it answers through the same K=0 step a fresh player shown only that
//! view would run, and its K-step readout starts from that.

use std::collections::BTreeMap;

/// When the seat under test forgets its recurrent memory H.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryReset4 {
    /// Never: H carries every observation of the game, as trained.
    #[default]
    Off,
    /// At every view: each observation update starts from zero, so a decision
    /// reads H = K0(0, its own view) and nothing observed before.
    Decision,
    /// At the first view of each new turn number: H carries the observations
    /// of the current turn only.
    Turn,
}

impl MemoryReset4 {
    /// The flag's word, as `run.json` and the summary record it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Decision => "decision",
            Self::Turn => "turn",
        }
    }

    /// Whether `seat`'s observation of a view of `turn` starts from zero H,
    /// noting the turn in `turns` (the turn each seat last observed). Off never
    /// starts afresh and notes nothing.
    pub(super) fn starts_fresh(self, turns: &mut BTreeMap<u8, u32>, seat: u8, turn: u32) -> bool {
        match self {
            Self::Off => false,
            Self::Decision => true,
            Self::Turn => turns.insert(seat, turn) != Some(turn),
        }
    }

    /// Whether a decision of `seat` at `turn` must forget H that no
    /// observation of this turn has reset yet: a host that answers without
    /// observing first. Only `Turn` resets at a decision; `Decision` resets
    /// at the observation the decision reads.
    pub(super) fn decision_starts_fresh(
        self,
        turns: &mut BTreeMap<u8, u32>,
        seat: u8,
        turn: u32,
    ) -> bool {
        self == Self::Turn && self.starts_fresh(turns, seat, turn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_never_forgets_and_notes_nothing() {
        let mut turns = BTreeMap::new();
        for turn in [1, 1, 2, 3] {
            assert!(!MemoryReset4::Off.starts_fresh(&mut turns, 0, turn));
            assert!(!MemoryReset4::Off.decision_starts_fresh(&mut turns, 0, turn));
        }
        assert!(turns.is_empty());
    }

    #[test]
    fn decision_forgets_at_every_observation_and_never_between_it_and_the_answer() {
        let mut turns = BTreeMap::new();
        for turn in [1, 1, 2] {
            assert!(MemoryReset4::Decision.starts_fresh(&mut turns, 0, turn));
            assert!(!MemoryReset4::Decision.decision_starts_fresh(&mut turns, 0, turn));
        }
    }

    /// Turn resets exactly at the first view of each new turn number, per seat.
    #[test]
    fn turn_forgets_exactly_at_turn_changes() {
        let mut turns = BTreeMap::new();
        let views = [
            (0, 1),
            (0, 1),
            (1, 1),
            (0, 2),
            (0, 2),
            (1, 2),
            (1, 2),
            (0, 3),
            (0, 2),
        ];
        let fresh: Vec<bool> = views
            .iter()
            .map(|&(seat, turn)| MemoryReset4::Turn.starts_fresh(&mut turns, seat, turn))
            .collect();
        assert_eq!(
            fresh,
            [true, false, true, true, false, true, false, true, true]
        );
        // A decision after its turn's first observation keeps H.
        assert!(!MemoryReset4::Turn.decision_starts_fresh(&mut turns, 0, 2));
        // One without an observation of a new turn forgets it.
        assert!(MemoryReset4::Turn.decision_starts_fresh(&mut turns, 1, 4));
    }

    #[test]
    fn the_flag_words_round_trip() {
        for mode in [
            MemoryReset4::Off,
            MemoryReset4::Decision,
            MemoryReset4::Turn,
        ] {
            let word = serde_json::to_value(mode).unwrap();
            assert_eq!(word, mode.name());
            assert_eq!(serde_json::from_value::<MemoryReset4>(word).unwrap(), mode);
        }
    }
}
