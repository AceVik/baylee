//! Standing orders and the autopilot: per-phase skip preferences, the
//! phase-rail selection, and the "next phase" / "end turn" buttons.
//!
//! Everything here is a pure decision over the pending choice and the
//! view's phase — the renderer only has to draw the answers.

use baylee_engine::choice::{LegalActions, Pending};
use baylee_view::{Phase, Step};

/// One row of the phase rail: every step of a Magic turn, in order
/// (CR 500.1). The two main phases share `Step::Main` and are told apart
/// by their phase; the two combat damage steps share one row.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
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

/// Which of the two rails a button belongs to: the phases of *your own*
/// (or a teammate's) turns, or the phases of *opponents'* turns. Both
/// are priority controls — a red opponent-attackers row means "don't ask
/// me for blocks", a red own-upkeep row means "don't ask me there".
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum RailSide {
    /// Phases of your own and your teammates' turns.
    Mine,
    /// Phases of opponents' turns.
    Theirs,
}

impl RailSide {
    /// Both sides, in rail order (opponents on top, you at the bottom).
    pub const BOTH: [Self; 2] = [Self::Theirs, Self::Mine];

    /// Array index.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Mine => 0,
            Self::Theirs => 1,
        }
    }
}

/// Per-step standing orders for both rails: green means "I want priority
/// here", red means "skip — take no action and move on".
///
/// Everything defaults to green, which is the honest default: a client
/// that auto-passes without being asked loses games its player never
/// agreed to lose.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PhaseOrders {
    /// `true` = red (skip) at that rail index, per side.
    skip: [[bool; 12]; 2],
    /// The keyboard-selected button (side + row), for Shift+W/S + Space.
    ///
    /// Never stored: which button the keyboard is resting on is a property
    /// of this screen right now, not of the account.
    #[serde(skip)]
    selected: Option<(RailSide, RailRow)>,
}

impl PhaseOrders {
    /// Toggles a button between green (priority) and red (skip).
    pub fn toggle(&mut self, side: RailSide, row: RailRow) {
        let i = row.index();
        self.skip[side.index()][i] = !self.skip[side.index()][i];
    }

    /// Whether a button is red (skip).
    #[must_use]
    pub fn is_skipped(&self, side: RailSide, row: RailRow) -> bool {
        self.skip[side.index()][row.index()]
    }

    /// Whether the given (phase, step) falls on a red row, given whose
    /// turn it currently is.
    #[must_use]
    pub fn is_skipped_at(&self, active_is_mine: bool, phase: Phase, step: Step) -> bool {
        let side = if active_is_mine {
            RailSide::Mine
        } else {
            RailSide::Theirs
        };
        self.is_skipped(side, RailRow::current(phase, step))
    }

    /// The keyboard-selected button, if any.
    #[must_use]
    pub const fn selected(&self) -> Option<(RailSide, RailRow)> {
        self.selected
    }

    /// Clears the keyboard selection.
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// Moves the keyboard selection by `delta` buttons over the flattened
    /// rail (theirs' twelve rows first, then yours), wrapping; with no
    /// selection yet, starts at the first row of your rail.
    pub fn move_selection(&mut self, delta: i32) {
        let Some((side, row)) = self.selected else {
            // No selection yet: start at the first row of your rail.
            self.selected = Some((RailSide::Mine, RailRow::Untap));
            return;
        };
        let base = match side {
            RailSide::Theirs => 0,
            RailSide::Mine => RAIL_ROWS.len() as i32,
        };
        let flat = base + row.index() as i32;
        let next = (flat + delta).rem_euclid((RAIL_ROWS.len() * 2) as i32);
        let (side, row) = if next < RAIL_ROWS.len() as i32 {
            (RailSide::Theirs, RAIL_ROWS[next as usize])
        } else {
            (RailSide::Mine, RAIL_ROWS[next as usize - RAIL_ROWS.len()])
        };
        self.selected = Some((side, row));
    }

    /// One rail as (row, skipped) pairs, for drawing.
    pub fn rows_for(&self, side: RailSide) -> impl Iterator<Item = (RailRow, bool)> + '_ {
        RAIL_ROWS
            .into_iter()
            .map(move |r| (r, self.is_skipped(side, r)))
    }

    /// Whether two order sets are identical (cheap change detection).
    #[must_use]
    pub fn same_as(&self, other: &Self) -> bool {
        self.skip == other.skip && self.selected == other.selected
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

/// Where the game is, from the local seat's point of view.
///
/// Bundled rather than passed as four loose flags, because they are always
/// read together and swapping `mine` for `active_is_mine` is a bug no
/// signature would catch.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Situation {
    /// Whether the pending choice is the local seat's to answer.
    pub mine: bool,
    /// Whether the active player is the local seat or a teammate.
    pub active_is_mine: bool,
    /// The phase the view is in.
    pub phase: Phase,
    /// The step the view is in.
    pub step: Step,
}

/// The standing-order decision: given the pending choice, where the game
/// is, the per-step rail, the account's automation rules and an optional
/// autopilot, what is answered without asking?
///
/// The rule of thumb: a red row means "I do nothing here" (pass, no
/// attackers, no blockers), the autopilot means "fast-forward to the
/// boundary, but never make a real decision for me", and the rules are the
/// same promise for the questions that are not really questions.
///
/// One line that is deliberately *not* here: `skip_opponent_turns` passes
/// priority on an opponent's turn and nothing more. It never declines a
/// block. Losing a creature you would have blocked with is exactly the kind
/// of decision a client must not make for its player.
#[must_use]
pub fn auto_answer(
    pending: &Pending,
    at: Situation,
    orders: &PhaseOrders,
    rules: &crate::prefs::AutoRules,
    pilot: Option<&AutoPilot>,
) -> AutoAnswer {
    if !at.mine {
        return AutoAnswer::None;
    }
    let skipped = orders.is_skipped_at(at.active_is_mine, at.phase, at.step);
    let quiet_turn = rules.skip_opponent_turns && !at.active_is_mine;
    match pending {
        Pending::Priority { legal, .. }
            if skipped
                || quiet_turn
                || pilot.is_some()
                || (rules.pass_when_nothing_to_do && nothing_to_do(legal)) =>
        {
            AutoAnswer::Pass
        }
        Pending::ChooseAttackers { attackers, .. }
            if skipped
                || matches!(pilot, Some(AutoPilot::ToNextTurn { .. }))
                || (rules.skip_empty_attacks && attackers.is_empty()) =>
        {
            AutoAnswer::DeclareNoAttackers
        }
        Pending::ChooseBlockers { blockers, .. }
            if skipped || (rules.skip_empty_blocks && blockers.is_empty()) =>
        {
            AutoAnswer::DeclareNoBlockers
        }
        _ => AutoAnswer::None,
    }
}

/// Whether a priority window offers the seat nothing at all.
///
/// "Nothing" is meant literally: no land to play, no spell to cast, no
/// ability to activate, nothing to suspend. Mana abilities do not count —
/// floating mana with nothing to spend it on is not something to do, and
/// counting it would mean the rule never fires for anyone holding a land.
fn nothing_to_do(legal: &LegalActions) -> bool {
    legal.lands.is_empty()
        && legal.castable.is_empty()
        && legal.abilities.is_empty()
        && legal.suspendable.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefs::AutoRules;
    use baylee_core::ids::PlayerId;
    use baylee_engine::choice::LegalActions;

    fn at(mine: bool, active_is_mine: bool, phase: Phase, step: Step) -> Situation {
        Situation {
            mine,
            active_is_mine,
            phase,
            step,
        }
    }

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
        for side in RailSide::BOTH {
            for (row, skipped) in orders.rows_for(side) {
                assert!(!skipped, "{side:?}/{row:?} must default to green");
            }
        }
        let answer = auto_answer(
            &priority_pending(),
            at(true, true, Phase::FirstMain, Step::Main),
            &orders,
            &AutoRules::default(),
            None,
        );
        assert_eq!(answer, AutoAnswer::None);
    }

    #[test]
    fn a_red_row_passes_and_stays_out_of_combat() {
        let mut orders = PhaseOrders::default();
        orders.toggle(RailSide::Mine, RailRow::Attackers);
        orders.toggle(RailSide::Mine, RailRow::Blockers);
        orders.toggle(RailSide::Mine, RailRow::Damage);
        assert_eq!(
            auto_answer(
                &priority_pending(),
                at(true, true, Phase::Combat, Step::CombatDamage),
                &orders,
                &AutoRules::default(),
                None,
            ),
            AutoAnswer::Pass
        );
        assert_eq!(
            auto_answer(
                &Pending::ChooseAttackers {
                    player: PlayerId::new(0),
                    attackers: vec![],
                    defenders: Vec::new(),
                },
                at(true, true, Phase::Combat, Step::DeclareAttackers),
                &orders,
                &AutoRules::default(),
                None,
            ),
            AutoAnswer::DeclareNoAttackers
        );
        assert_eq!(
            auto_answer(
                &Pending::ChooseBlockers {
                    player: PlayerId::new(0),
                    blockers: vec![],
                    attacker: PlayerId::new(1),
                },
                at(true, true, Phase::Combat, Step::DeclareBlockers),
                &orders,
                &AutoRules::default(),
                None,
            ),
            AutoAnswer::DeclareNoBlockers
        );
        // …but a green row in the same phase is untouched.
        assert_eq!(
            auto_answer(
                &priority_pending(),
                at(true, true, Phase::Combat, Step::CombatBegin),
                &orders,
                &AutoRules::default(),
                None,
            ),
            AutoAnswer::None
        );
        // …and the opponent rail is a separate switch: my red rows do not
        // skip the opponent's turn.
        assert_eq!(
            auto_answer(
                &priority_pending(),
                at(true, false, Phase::Combat, Step::CombatDamage),
                &orders,
                &AutoRules::default(),
                None,
            ),
            AutoAnswer::None
        );
    }

    #[test]
    fn a_red_opponent_blockers_row_declines_to_block_on_their_turn() {
        let mut orders = PhaseOrders::default();
        orders.toggle(RailSide::Theirs, RailRow::Blockers);
        assert_eq!(
            auto_answer(
                &Pending::ChooseBlockers {
                    player: PlayerId::new(0),
                    blockers: vec![],
                    attacker: PlayerId::new(1),
                },
                at(true, false, Phase::Combat, Step::DeclareBlockers),
                &orders,
                &AutoRules::default(),
                None,
            ),
            AutoAnswer::DeclareNoBlockers
        );
        // On MY turn the same row is green and I am asked.
        assert_eq!(
            auto_answer(
                &Pending::ChooseBlockers {
                    player: PlayerId::new(0),
                    blockers: vec![],
                    attacker: PlayerId::new(1),
                },
                at(true, true, Phase::Combat, Step::DeclareBlockers),
                &orders,
                &AutoRules::default(),
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
                at(true, true, Phase::FirstMain, Step::Main),
                &orders,
                &AutoRules::default(),
                Some(&pilot)
            ),
            AutoAnswer::Pass
        );
        // Attackers are a decision: ToNextPhase leaves them to the player…
        assert_eq!(
            auto_answer(
                &Pending::ChooseAttackers {
                    player: PlayerId::new(0),
                    attackers: vec![],
                    defenders: Vec::new(),
                },
                at(true, true, Phase::Combat, Step::DeclareAttackers),
                &orders,
                &AutoRules::default(),
                Some(&pilot),
            ),
            AutoAnswer::None
        );
        // …while End Turn declines the attack and stops at the boundary.
        let end_turn = AutoPilot::ToNextTurn { from_turn: 3 };
        assert_eq!(
            auto_answer(
                &Pending::ChooseAttackers {
                    player: PlayerId::new(0),
                    attackers: vec![],
                    defenders: Vec::new(),
                },
                at(true, true, Phase::Combat, Step::DeclareAttackers),
                &orders,
                &AutoRules::default(),
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
    fn selection_walks_both_rails_and_wraps() {
        let mut orders = PhaseOrders::default();
        assert_eq!(orders.selected(), None);
        orders.move_selection(-1);
        assert_eq!(
            orders.selected(),
            Some((RailSide::Mine, RailRow::Untap)),
            "starts at your rail's first row"
        );
        orders.move_selection(-1);
        assert_eq!(
            orders.selected(),
            Some((RailSide::Theirs, RailRow::Cleanup)),
            "wraps upward into the opponent rail"
        );
        orders.move_selection(1);
        assert_eq!(orders.selected(), Some((RailSide::Mine, RailRow::Untap)));
        orders.move_selection(4);
        assert_eq!(
            orders.selected(),
            Some((RailSide::Mine, RailRow::CombatBegin))
        );
        orders.clear_selection();
        assert_eq!(orders.selected(), None);
    }

    #[test]
    fn nothing_is_answered_for_someone_elses_choice() {
        let mut orders = PhaseOrders::default();
        orders.toggle(RailSide::Mine, RailRow::Main1);
        let pilot = AutoPilot::ToNextPhase {
            from: Phase::FirstMain,
        };
        assert_eq!(
            auto_answer(
                &priority_pending(),
                at(false, true, Phase::FirstMain, Step::Main),
                &orders,
                &AutoRules::default(),
                Some(&pilot)
            ),
            AutoAnswer::None
        );
    }

    #[test]
    fn a_window_offering_nothing_passes_itself_only_once_asked_to() {
        let orders = PhaseOrders::default();
        let empty = priority_pending();
        let mut rules = AutoRules::default();
        // Off by default: an empty window is still the player's to pass.
        assert_eq!(
            auto_answer(
                &empty,
                at(true, true, Phase::FirstMain, Step::Main),
                &orders,
                &rules,
                None
            ),
            AutoAnswer::None
        );
        rules.pass_when_nothing_to_do = true;
        assert_eq!(
            auto_answer(
                &empty,
                at(true, true, Phase::FirstMain, Step::Main),
                &orders,
                &rules,
                None
            ),
            AutoAnswer::Pass
        );
        // One castable spell is something to do, and the player is asked.
        let castable = Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                can_pass: true,
                lands: vec![],
                castable: vec![baylee_core::ids::ObjectId::new(1, 0)],
                mana_abilities: vec![],
                abilities: vec![],
                suspendable: vec![],
            }),
        };
        assert_eq!(
            auto_answer(
                &castable,
                at(true, true, Phase::FirstMain, Step::Main),
                &orders,
                &rules,
                None
            ),
            AutoAnswer::None
        );
    }

    #[test]
    fn floating_mana_is_not_something_to_do() {
        // A seat holding an untapped land can always make mana, so counting
        // mana abilities would mean the rule never fires for anyone.
        let orders = PhaseOrders::default();
        let rules = AutoRules {
            pass_when_nothing_to_do: true,
            ..AutoRules::default()
        };
        let only_mana = Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                can_pass: true,
                lands: vec![],
                castable: vec![],
                mana_abilities: vec![baylee_core::ids::ObjectId::new(1, 0)],
                abilities: vec![],
                suspendable: vec![],
            }),
        };
        assert_eq!(
            auto_answer(
                &only_mana,
                at(true, true, Phase::FirstMain, Step::Main),
                &orders,
                &rules,
                None
            ),
            AutoAnswer::Pass
        );
    }

    #[test]
    fn skipping_opponent_turns_never_skips_a_block() {
        let orders = PhaseOrders::default();
        let rules = AutoRules {
            skip_opponent_turns: true,
            ..AutoRules::default()
        };
        // Priority on their turn: passed.
        assert_eq!(
            auto_answer(
                &priority_pending(),
                at(true, false, Phase::FirstMain, Step::Main),
                &orders,
                &rules,
                None
            ),
            AutoAnswer::Pass
        );
        // A block on their turn: still mine to make, and the whole point of
        // the rule being priority-only.
        let blocks = Pending::ChooseBlockers {
            player: PlayerId::new(0),
            blockers: vec![baylee_engine::choice::BlockOption {
                blocker: baylee_core::ids::ObjectId::new(1, 0),
                attackers: vec![baylee_core::ids::ObjectId::new(2, 0)],
            }],
            attacker: PlayerId::new(1),
        };
        assert_eq!(
            auto_answer(
                &blocks,
                at(true, false, Phase::Combat, Step::DeclareBlockers),
                &orders,
                &rules,
                None
            ),
            AutoAnswer::None
        );
        // And my own turn is untouched.
        assert_eq!(
            auto_answer(
                &priority_pending(),
                at(true, true, Phase::FirstMain, Step::Main),
                &orders,
                &rules,
                None
            ),
            AutoAnswer::None
        );
    }

    #[test]
    fn an_empty_combat_question_can_be_answered_for_you() {
        let orders = PhaseOrders::default();
        let rules = AutoRules {
            skip_empty_attacks: true,
            skip_empty_blocks: true,
            ..AutoRules::default()
        };
        let no_attackers = Pending::ChooseAttackers {
            player: PlayerId::new(0),
            attackers: vec![],
            defenders: Vec::new(),
        };
        assert_eq!(
            auto_answer(
                &no_attackers,
                at(true, true, Phase::Combat, Step::DeclareAttackers),
                &orders,
                &rules,
                None
            ),
            AutoAnswer::DeclareNoAttackers
        );
        // But a creature that *can* attack is always the player's call.
        let could_attack = Pending::ChooseAttackers {
            player: PlayerId::new(0),
            attackers: vec![baylee_core::ids::ObjectId::new(3, 0)],
            defenders: Vec::new(),
        };
        assert_eq!(
            auto_answer(
                &could_attack,
                at(true, true, Phase::Combat, Step::DeclareAttackers),
                &orders,
                &rules,
                None
            ),
            AutoAnswer::None
        );
    }
}
