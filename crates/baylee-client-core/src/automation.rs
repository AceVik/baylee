//! Standing orders and the autopilot: per-phase skip preferences, the
//! phase-rail selection, and the "next phase" / "end turn" buttons.
//!
//! Everything here is a pure decision over the pending choice and the
//! view's phase — the renderer only has to draw the answers.

use baylee_engine::choice::Pending;
use baylee_view::{Phase, Step};

/// One row of the phase rail: every step of a Magic turn, in order
/// (CR 500.1). The two main phases share `Step::Main` and are told apart
/// by their phase; the two combat damage steps share one row.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RailRow {
    /// Untap step (no priority in rules — informational).
    Untap,
    /// Upkeep step.
    Upkeep,
    /// Draw step.
    Draw,
    /// Precombat main phase.
    Main1,
    /// Beginning of combat step.
    CombatBegin,
    /// Declare attackers step.
    Attackers,
    /// Declare blockers step.
    Blockers,
    /// Combat damage steps (first-strike and regular share the row).
    Damage,
    /// End of combat step.
    CombatEnd,
    /// Postcombat main phase.
    Main2,
    /// End step.
    EndStep,
    /// Cleanup step (no priority in rules — informational).
    Cleanup,
}

/// The rail's rows, in turn order.
pub const RAIL_ROWS: [RailRow; 12] = [
    RailRow::Untap,
    RailRow::Upkeep,
    RailRow::Draw,
    RailRow::Main1,
    RailRow::CombatBegin,
    RailRow::Attackers,
    RailRow::Blockers,
    RailRow::Damage,
    RailRow::CombatEnd,
    RailRow::Main2,
    RailRow::EndStep,
    RailRow::Cleanup,
];

impl RailRow {
    /// The rail label.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Untap => "Untap",
            Self::Upkeep => "Upkeep",
            Self::Draw => "Draw",
            Self::Main1 => "Main 1",
            Self::CombatBegin => "Begin Combat",
            Self::Attackers => "Attackers",
            Self::Blockers => "Blockers",
            Self::Damage => "Damage",
            Self::CombatEnd => "End of Combat",
            Self::Main2 => "Main 2",
            Self::EndStep => "End Step",
            Self::Cleanup => "Cleanup",
        }
    }

    /// The row a (phase, step) pair belongs to.
    #[must_use]
    pub const fn current(phase: Phase, step: Step) -> Self {
        match (phase, step) {
            (Phase::Beginning, Step::Untap) => Self::Untap,
            (Phase::Beginning, Step::Upkeep) => Self::Upkeep,
            (Phase::Beginning, _) => Self::Draw,
            (Phase::FirstMain, _) => Self::Main1,
            (Phase::Combat, Step::DeclareAttackers) => Self::Attackers,
            (Phase::Combat, Step::DeclareBlockers) => Self::Blockers,
            (Phase::Combat, Step::CombatDamageFirst | Step::CombatDamage) => Self::Damage,
            (Phase::Combat, Step::CombatEnd) => Self::CombatEnd,
            (Phase::Combat, _) => Self::CombatBegin,
            (Phase::SecondMain, _) => Self::Main2,
            (Phase::Ending, Step::Cleanup) => Self::Cleanup,
            (Phase::Ending, _) => Self::EndStep,
        }
    }

    /// Index in [`RAIL_ROWS`].
    ///
    /// # Panics
    /// Never, in practice: every row is on the rail.
    #[must_use]
    pub fn index(self) -> usize {
        RAIL_ROWS
            .iter()
            .position(|r| *r == self)
            .expect("every row is on the rail")
    }
}

/// Per-step standing orders: green means "I want priority here", red
/// means "skip — take no action and move on".
///
/// Everything defaults to green, which is the honest default: a client
/// that auto-passes without being asked loses games its player never
/// agreed to lose.
#[derive(Clone, Debug, Default)]
pub struct PhaseOrders {
    /// `true` = red (skip) at that rail index.
    skip: [bool; 12],
    /// The keyboard-selected rail row (for Shift+W/S + Space toggling).
    selected: Option<RailRow>,
}

impl PhaseOrders {
    /// Toggles a row between green (priority) and red (skip).
    pub fn toggle(&mut self, row: RailRow) {
        let i = row.index();
        self.skip[i] = !self.skip[i];
    }

    /// Whether a row is red (skip).
    #[must_use]
    pub fn is_skipped(&self, row: RailRow) -> bool {
        self.skip[row.index()]
    }

    /// Whether the given (phase, step) falls on a red row.
    #[must_use]
    pub fn is_skipped_at(&self, phase: Phase, step: Step) -> bool {
        self.is_skipped(RailRow::current(phase, step))
    }

    /// The keyboard-selected rail row, if any.
    #[must_use]
    pub const fn selected(&self) -> Option<RailRow> {
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
            Some(row) => (row.index() as i32 + delta).rem_euclid(RAIL_ROWS.len() as i32) as usize,
        };
        self.selected = Some(RAIL_ROWS[next]);
    }

    /// The rail as (row, skipped) pairs, for drawing.
    pub fn rows(&self) -> impl Iterator<Item = (RailRow, bool)> + '_ {
        RAIL_ROWS.into_iter().map(|r| (r, self.is_skipped(r)))
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
/// for, the view's (phase, step), the per-step orders, and an optional
/// autopilot, what is answered automatically?
///
/// The rule of thumb: a red row means "I do nothing here" (pass, no
/// attackers, no blockers), the autopilot means "fast-forward to the
/// boundary, but never make a real decision for me".
#[must_use]
pub fn auto_answer(
    pending: &Pending,
    mine: bool,
    phase: Phase,
    step: Step,
    orders: &PhaseOrders,
    pilot: Option<&AutoPilot>,
) -> AutoAnswer {
    if !mine {
        return AutoAnswer::None;
    }
    let skipped = orders.is_skipped_at(phase, step);
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
        for (row, skipped) in orders.rows() {
            assert!(!skipped, "{row:?} must default to green");
        }
        let answer = auto_answer(
            &priority_pending(),
            true,
            Phase::FirstMain,
            Step::Main,
            &orders,
            None,
        );
        assert_eq!(answer, AutoAnswer::None);
    }

    #[test]
    fn a_red_row_passes_and_stays_out_of_combat() {
        let mut orders = PhaseOrders::default();
        orders.toggle(RailRow::Attackers);
        orders.toggle(RailRow::Blockers);
        orders.toggle(RailRow::Damage);
        assert_eq!(
            auto_answer(
                &priority_pending(),
                true,
                Phase::Combat,
                Step::CombatDamage,
                &orders,
                None,
            ),
            AutoAnswer::Pass
        );
        assert_eq!(
            auto_answer(
                &Pending::ChooseAttackers {
                    player: PlayerId::new(0)
                },
                true,
                Phase::Combat,
                Step::DeclareAttackers,
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
                Step::DeclareBlockers,
                &orders,
                None,
            ),
            AutoAnswer::DeclareNoBlockers
        );
        // …but a green row in the same phase is untouched.
        assert_eq!(
            auto_answer(
                &priority_pending(),
                true,
                Phase::Combat,
                Step::CombatBegin,
                &orders,
                None,
            ),
            AutoAnswer::None
        );
        // …and so is everything in Main 1.
        assert_eq!(
            auto_answer(
                &priority_pending(),
                true,
                Phase::FirstMain,
                Step::Main,
                &orders,
                None,
            ),
            AutoAnswer::None
        );
    }

    #[test]
    fn the_rail_maps_every_step_to_its_row() {
        assert_eq!(
            RailRow::current(Phase::Beginning, Step::Untap),
            RailRow::Untap
        );
        assert_eq!(
            RailRow::current(Phase::Beginning, Step::Upkeep),
            RailRow::Upkeep
        );
        assert_eq!(
            RailRow::current(Phase::Beginning, Step::Draw),
            RailRow::Draw
        );
        assert_eq!(
            RailRow::current(Phase::FirstMain, Step::Main),
            RailRow::Main1
        );
        assert_eq!(
            RailRow::current(Phase::Combat, Step::CombatBegin),
            RailRow::CombatBegin
        );
        assert_eq!(
            RailRow::current(Phase::Combat, Step::DeclareAttackers),
            RailRow::Attackers
        );
        assert_eq!(
            RailRow::current(Phase::Combat, Step::DeclareBlockers),
            RailRow::Blockers
        );
        assert_eq!(
            RailRow::current(Phase::Combat, Step::CombatDamageFirst),
            RailRow::Damage,
            "first-strike damage shares the damage row"
        );
        assert_eq!(
            RailRow::current(Phase::Combat, Step::CombatDamage),
            RailRow::Damage
        );
        assert_eq!(
            RailRow::current(Phase::Combat, Step::CombatEnd),
            RailRow::CombatEnd
        );
        assert_eq!(
            RailRow::current(Phase::SecondMain, Step::Main),
            RailRow::Main2
        );
        assert_eq!(RailRow::current(Phase::Ending, Step::End), RailRow::EndStep);
        assert_eq!(
            RailRow::current(Phase::Ending, Step::Cleanup),
            RailRow::Cleanup
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
                Step::Main,
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
                Step::DeclareAttackers,
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
                Step::DeclareAttackers,
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
        assert_eq!(orders.selected(), Some(RailRow::Untap));
        orders.move_selection(-1);
        assert_eq!(orders.selected(), Some(RailRow::Cleanup), "wraps upward");
        orders.move_selection(1);
        assert_eq!(orders.selected(), Some(RailRow::Untap));
        orders.move_selection(4);
        assert_eq!(orders.selected(), Some(RailRow::CombatBegin));
        orders.clear_selection();
        assert_eq!(orders.selected(), None);
    }

    #[test]
    fn nothing_is_answered_for_someone_elses_choice() {
        let mut orders = PhaseOrders::default();
        orders.toggle(RailRow::Main1);
        let pilot = AutoPilot::ToNextPhase {
            from: Phase::FirstMain,
        };
        assert_eq!(
            auto_answer(
                &priority_pending(),
                false,
                Phase::FirstMain,
                Step::Main,
                &orders,
                Some(&pilot)
            ),
            AutoAnswer::None
        );
    }
}
