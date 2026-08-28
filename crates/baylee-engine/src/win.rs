//! Game results and win/lose evaluation.

use baylee_core::ids::PlayerId;
use serde::{Deserialize, Serialize};

/// The final result of a game.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct GameResult {
    /// The winner (`None` = draw).
    pub winner: Option<PlayerId>,
    /// Why the game ended.
    pub reason: EndReason,
}

/// Why the game ended.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum EndReason {
    /// All opponents lost (CR 104.1).
    LastPlayerStanding,
    /// A player won by effect (M2).
    EffectWin,
    /// All remaining players drew (e.g. mandatory loop with `CompRulesDraw`).
    Draw,
}
