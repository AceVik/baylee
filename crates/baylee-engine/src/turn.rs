//! Phases, steps, and turn bookkeeping.
//!
//! Only the data model lives here for now; the turn engine (turn-based
//! actions, priority passes, duration cleanup) arrives in M1.S2.

use baylee_core::ids::PlayerId;
use serde::{Deserialize, Serialize};

/// The five phases of a turn (CR 505.1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Phase {
    /// Beginning phase.
    Beginning,
    /// First main phase.
    FirstMain,
    /// Combat phase.
    Combat,
    /// Second main phase.
    SecondMain,
    /// Ending phase.
    Ending,
}

/// The steps of a turn (CR 505.1); `Main` covers both main phases.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Step {
    /// Untap step.
    Untap,
    /// Upkeep step.
    Upkeep,
    /// Draw step.
    Draw,
    /// A main phase (no step boundary in rules terms).
    Main,
    /// Beginning of combat.
    CombatBegin,
    /// Declare attackers.
    DeclareAttackers,
    /// Declare blockers.
    DeclareBlockers,
    /// First-strike combat damage.
    CombatDamageFirst,
    /// Regular combat damage.
    CombatDamage,
    /// End of combat.
    CombatEnd,
    /// End step.
    End,
    /// Cleanup step.
    Cleanup,
}

impl Step {
    /// The phase a step belongs to.
    #[must_use]
    pub const fn phase(self) -> Phase {
        match self {
            Step::Untap | Step::Upkeep | Step::Draw => Phase::Beginning,
            Step::Main => Phase::FirstMain, // context-dependent; see TurnInfo
            Step::CombatBegin
            | Step::DeclareAttackers
            | Step::DeclareBlockers
            | Step::CombatDamageFirst
            | Step::CombatDamage
            | Step::CombatEnd => Phase::Combat,
            Step::End | Step::Cleanup => Phase::Ending,
        }
    }
}

/// Where the game currently is.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TurnInfo {
    /// Turn number (1-based).
    pub number: u32,
    /// Active player.
    pub active: PlayerId,
    /// Current phase.
    pub phase: Phase,
    /// Current step.
    pub step: Step,
}

impl TurnInfo {
    /// The start of the game: turn 1, active player's beginning phase.
    #[must_use]
    pub const fn new(active: PlayerId) -> Self {
        Self {
            number: 1,
            active,
            phase: Phase::Beginning,
            step: Step::Untap,
        }
    }
}
