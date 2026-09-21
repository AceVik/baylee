//! Phases, steps, and turn bookkeeping.
//!
//! Only the data model lives here for now; the turn engine (turn-based
//! actions, priority passes, duration cleanup) arrives in M1.S2.

use baylee_core::ids::PlayerId;
use serde::{Deserialize, Serialize};

/// The five phases of a turn (CR 500.1).
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

/// The steps of a turn; `Main` covers both main phases.
///
/// Three phases have steps and the citation is one apiece: the beginning
/// phase (CR 501.1), the combat phase (CR 506.1) and the ending phase
/// (CR 512.1). The two main phases have none — CR 505.2 is what says so, and
/// it was the number written here for both of these enums.
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

/// The game's day/night designation (CR 730.1).
///
/// A designation belongs to the *game*, not to a player or a permanent, so
/// it sits beside the monarch on [`crate::state::GameState`] rather than on
/// anything in a zone. It is wrapped in an `Option` there because a game
/// starts with **neither** designation, and CR 730.1's last sentence is
/// what makes that an `Option` rather than a third variant: once the game
/// has become day or night it has exactly one of the two from that point
/// forward, so the field only ever goes `None -> Some` and never back.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[repr(u8)]
pub enum DayNight {
    /// It is day.
    Day = 0,
    /// It is night.
    Night = 1,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every step belongs to the phase the rules put it in: the beginning
    /// phase has three (CR 501.1), combat has five (CR 506.1) and the ending
    /// phase two (CR 512.1). The two main phases have none at all — CR 505.2
    /// is what says so — which is why `Step::Main` is the one answer this
    /// function cannot give on its own.
    #[test]
    fn every_step_belongs_to_the_phase_the_rules_put_it_in() {
        const STEPS: [(Step, Phase); 12] = [
            (Step::Untap, Phase::Beginning),
            (Step::Upkeep, Phase::Beginning),
            (Step::Draw, Phase::Beginning),
            (Step::Main, Phase::FirstMain),
            (Step::CombatBegin, Phase::Combat),
            (Step::DeclareAttackers, Phase::Combat),
            (Step::DeclareBlockers, Phase::Combat),
            (Step::CombatDamageFirst, Phase::Combat),
            (Step::CombatDamage, Phase::Combat),
            (Step::CombatEnd, Phase::Combat),
            (Step::End, Phase::Ending),
            (Step::Cleanup, Phase::Ending),
        ];
        for (step, phase) in STEPS {
            assert_eq!(step.phase(), phase, "{step:?}");
        }
        // Six variants for the five steps CR 506.1 names: the combat damage
        // step happens twice when anything has first or double strike
        // (CR 510.4), and this enum gives that second one a name of its own
        // rather than a flag.
        assert_eq!(STEPS.iter().filter(|(_, p)| *p == Phase::Combat).count(), 6);
    }

    /// `Step::Main` answering `FirstMain` is a **limitation and not an
    /// answer**: both main phases are `Step::Main`, so the step alone cannot
    /// say which one it is and only [`TurnInfo`] knows. Pinned so that the
    /// day the two are told apart, this is the test that has to move.
    #[test]
    fn a_main_phase_cannot_be_told_from_its_step() {
        assert_eq!(Step::Main.phase(), Phase::FirstMain);
        let second = TurnInfo {
            number: 3,
            active: PlayerId::new(1),
            phase: Phase::SecondMain,
            step: Step::Main,
        };
        assert_eq!(
            second.step.phase(),
            Phase::FirstMain,
            "the step says the first main phase while the turn is in the second"
        );
        assert_eq!(
            second.phase,
            Phase::SecondMain,
            "which is why the phase is carried beside the step and not derived"
        );
    }

    /// A game starts in the first player's untap step on turn 1, which is
    /// the one piece of turn state nobody sets by hand.
    #[test]
    fn a_game_starts_where_the_rules_start_it() {
        let start = TurnInfo::new(PlayerId::new(1));
        assert_eq!(start.number, 1);
        assert_eq!(start.active, PlayerId::new(1));
        assert_eq!((start.phase, start.step), (Phase::Beginning, Step::Untap));
        assert_eq!(
            start.step.phase(),
            start.phase,
            "and the two agree from the first frame"
        );
    }
}
