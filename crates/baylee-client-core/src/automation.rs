//! Standing orders and the autopilot: per-phase skip preferences, the
//! phase-rail selection, and the "next phase" / "end turn" buttons.
//!
//! Everything here is a pure decision over the pending choice and the
//! view's phase — the renderer only has to draw the answers.

use baylee_engine::choice::Pending;
use baylee_view::Phase;

/// The phase rail's rows, in turn order.
pub const RAIL_PHASES: [Phase; 5] = [
    Phase::Beginning,
    Phase::FirstMain,
    Phase::Combat,
    Phase::SecondMain,
    Phase::Ending,
];

/// The rail label for a phase.
#[must_use]
pub const fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Beginning => "Begin",
        Phase::FirstMain => "Main 1",
        Phase::Combat => "Combat",
        Phase::SecondMain => "Main 2",
        Phase::Ending => "End",
    }
}

/// Index of a phase in [`RAIL_PHASES`].
///
/// # Panics
/// Never, in practice: every `Phase` variant is on the rail.
#[must_use]
pub fn phase_index(phase: Phase) -> usize {
    RAIL_PHASES
        .iter()
        .position(|p| *p == phase)
        .expect("every phase is on the rail")
}

/// Per-phase standing orders: green means "I want priority here", red
/// means "skip — take no action and move on".
///
/// Everything defaults to green, which is the honest default: a client
/// that auto-passes without being asked loses games its player never
/// agreed to lose.
#[derive(Clone, Debug, Default)]
pub struct PhaseOrders {
    /// `true` = red (skip) at that rail index.
    skip: [bool; 5],
    /// The keyboard-selected rail row (for Shift+W/S + Space toggling).
    selected: Option<Phase>,
}

impl PhaseOrders {
    /// Toggles a phase between green (priority) and red (skip).
    pub fn toggle(&mut self, phase: Phase) {
        let i = phase_index(phase);
        self.skip[i] = !self.skip[i];
    }

    /// Whether the phase is red (skip).
    #[must_use]
    pub fn is_skipped(&self, phase: Phase) -> bool {
        self.skip[phase_index(phase)]
    }

    /// The keyboard-selected rail row, if any.
    #[must_use]
    pub const fn selected(&self) -> Option<Phase> {
        self.selected
    }

    /// Clears the keyboard selection.
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// Moves the keyboard selection by `delta` rail rows, wrapping; with
    /// no selection yet, starts at the first row.
    pub fn move_selection(&mut self, delta: i32) {
        let next = match self.selected {
            None => 0,
            Some(phase) => {
                (phase_index(phase) as i32 + delta).rem_euclid(RAIL_PHASES.len() as i32) as usize
            }
        };
        self.selected = Some(RAIL_PHASES[next]);
    }

    /// The rail as (phase, skipped) pairs, for drawing.
    pub fn rows(&self) -> impl Iterator<Item = (Phase, bool)> + '_ {
        RAIL_PHASES.into_iter().map(|p| (p, self.is_skipped(p)))
    }
}

/// The autopilot engaged by the rail buttons: auto-answer until a
/// boundary, then hand control back.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AutoPilot {
    /// Pass priority until the phase changes (TAB / "Next"). Decisions
    /// that are not plain priority still go to the player.
    ToNextPhase {
        /// The phase the button was pressed in.
        from: Phase,
    },
    /// Pass priority (and declare no attackers) until the turn changes
    /// ("End turn"). Blockers and real decisions still go to the player.
    ToNextTurn {
        /// The turn number the button was pressed in.
        from_turn: u32,
    },
}

impl AutoPilot {
    /// Whether the boundary has been crossed and control returns.
    #[must_use]
    pub fn reached(&self, phase: Phase, turn: u32) -> bool {
        match self {
            Self::ToNextPhase { from } => phase != *from,
            Self::ToNextTurn { from_turn } => turn != *from_turn,
        }
    }
}

/// What the orders/autopilot answer on the player's behalf.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AutoAnswer {
    /// Nothing — the player decides.
    None,
    /// Pass priority.
    Pass,
    /// Declare no attackers.
    DeclareNoAttackers,
    /// Declare no blockers.
    DeclareNoBlockers,
}

/// The standing-order decision: given the pending choice and who it is
/// for, the view's phase, the per-phase orders, and an optional autopilot,
/// what is answered automatically?
///
/// The rule of thumb: a red phase means "I do nothing here" (pass, no
/// attackers, no blockers), the autopilot means "fast-forward to the
/// boundary, but never make a real decision for me".
#[must_use]
pub fn auto_answer(
    pending: &Pending,
    mine: bool,
    phase: Phase,
    orders: &PhaseOrders,
    pilot: Option<&AutoPilot>,
) -> AutoAnswer {
    if !mine {
        return AutoAnswer::None;
    }
    let skipped = orders.is_skipped(phase);
    match pending {
        Pending::Priority { .. } if skipped || pilot.is_some() => AutoAnswer::Pass,
        Pending::ChooseAttackers { .. }
            if skipped || matches!(pilot, Some(AutoPilot::ToNextTurn { .. })) =>
        {
            AutoAnswer::DeclareNoAttackers
        }
        Pending::ChooseBlockers { .. } if skipped => AutoAnswer::DeclareNoBlockers,
        _ => AutoAnswer::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;
    use baylee_engine::choice::LegalActions;

    fn priority_pending() -> Pending {
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                can_pass: true,
                lands: vec![],
                castable: vec![],
                mana_abilities: vec![],
                abilities: vec![],
                suspendable: vec![],
            }),
        }
    }

    #[test]
    fn everything_is_green_by_default_so_nothing_is_auto_answered() {
        let orders = PhaseOrders::default();
        for (phase, skipped) in orders.rows() {
            assert!(!skipped, "{phase:?} must default to green");
        }
        let answer = auto_answer(&priority_pending(), true, Phase::FirstMain, &orders, None);
        assert_eq!(answer, AutoAnswer::None);
    }

    #[test]
    fn a_red_phase_passes_and_stays_out_of_combat() {
        let mut orders = PhaseOrders::default();
        orders.toggle(Phase::Combat);
        assert_eq!(
            auto_answer(&priority_pending(), true, Phase::Combat, &orders, None),
            AutoAnswer::Pass
        );
        assert_eq!(
            auto_answer(
                &Pending::ChooseAttackers {
                    player: PlayerId::new(0)
                },
                true,
                Phase::Combat,
                &orders,
                None,
            ),
            AutoAnswer::DeclareNoAttackers
        );
        assert_eq!(
            auto_answer(
                &Pending::ChooseBlockers {
                    player: PlayerId::new(0),
                    attacker: PlayerId::new(1),
                },
                true,
                Phase::Combat,
                &orders,
                None,
            ),
            AutoAnswer::DeclareNoBlockers
        );
        // …but a green phase is untouched.
        assert_eq!(
            auto_answer(&priority_pending(), true, Phase::FirstMain, &orders, None),
            AutoAnswer::None
        );
    }

    #[test]
    fn autopilot_passes_but_never_makes_real_decisions() {
        let orders = PhaseOrders::default();
        let pilot = AutoPilot::ToNextPhase {
            from: Phase::FirstMain,
        };
        assert_eq!(
            auto_answer(
                &priority_pending(),
                true,
                Phase::FirstMain,
                &orders,
                Some(&pilot)
            ),
            AutoAnswer::Pass
        );
        // Attackers are a decision: ToNextPhase leaves them to the player…
        assert_eq!(
            auto_answer(
                &Pending::ChooseAttackers {
                    player: PlayerId::new(0)
                },
                true,
                Phase::Combat,
                &orders,
                Some(&pilot),
            ),
            AutoAnswer::None
        );
        // …while End Turn declines the attack and stops at the boundary.
        let end_turn = AutoPilot::ToNextTurn { from_turn: 3 };
        assert_eq!(
            auto_answer(
                &Pending::ChooseAttackers {
                    player: PlayerId::new(0)
                },
                true,
                Phase::Combat,
                &orders,
                Some(&end_turn),
            ),
            AutoAnswer::DeclareNoAttackers
        );
        assert!(end_turn.reached(Phase::Beginning, 4));
        assert!(!end_turn.reached(Phase::Ending, 3));
        assert!(
            AutoPilot::ToNextPhase {
                from: Phase::Combat
            }
            .reached(Phase::SecondMain, 3)
        );
    }

    #[test]
    fn selection_walks_the_rail_and_wraps() {
        let mut orders = PhaseOrders::default();
        assert_eq!(orders.selected(), None);
        orders.move_selection(-1);
        assert_eq!(orders.selected(), Some(Phase::Beginning));
        orders.move_selection(-1);
        assert_eq!(orders.selected(), Some(Phase::Ending), "wraps upward");
        orders.move_selection(1);
        assert_eq!(orders.selected(), Some(Phase::Beginning));
        orders.move_selection(1);
        assert_eq!(orders.selected(), Some(Phase::FirstMain));
        orders.clear_selection();
        assert_eq!(orders.selected(), None);
    }

    #[test]
    fn nothing_is_answered_for_someone_elses_choice() {
        let mut orders = PhaseOrders::default();
        orders.toggle(Phase::FirstMain);
        let pilot = AutoPilot::ToNextPhase {
            from: Phase::FirstMain,
        };
        assert_eq!(
            auto_answer(
                &priority_pending(),
                false,
                Phase::FirstMain,
                &orders,
                Some(&pilot)
            ),
            AutoAnswer::None
        );
    }
}
