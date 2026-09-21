//! Standing orders and the autopilot: per-phase skip preferences, the
//! phase-rail selection, and the "next phase" / "end turn" buttons.
//!
//! Everything here is a pure decision over the pending choice and the
//! view's phase — the renderer only has to draw the answers.

use crate::i18n::Phrase;
use baylee_engine::choice::{LegalActions, Pending};
use baylee_view::{Phase, Step};

/// A remembered policy for one printed card ability, across printings and games.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AbilityOrder {
    /// Stable card and ability handle; different abilities remain independent.
    pub ability: baylee_core::ids::AbilityRef,
    /// Pass priority while this ability is on top of the stack.
    pub pass: bool,
    /// Answer optional yes/no questions; other choices still require input.
    pub answer: Option<baylee_engine::choice::StandingAnswer>,
}

impl AbilityOrder {
    /// Default manual policy for this ability.
    #[must_use]
    pub const fn manual(ability: baylee_core::ids::AbilityRef) -> Self {
        Self {
            ability,
            pass: false,
            answer: None,
        }
    }

    /// One atomic update, so disabling a rule cannot consume an old answer.
    #[must_use]
    pub const fn action(self) -> baylee_engine::choice::PlayerAction {
        baylee_engine::choice::PlayerAction::SetAbilityPolicy {
            ability: self.ability,
            pass: self.pass,
            answer: self.answer,
        }
    }
}

/// Find an ability's policy without conflating its source's other abilities.
#[must_use]
pub fn ability_order(
    orders: &[AbilityOrder],
    ability: baylee_core::ids::AbilityRef,
) -> AbilityOrder {
    orders
        .iter()
        .find(|order| order.ability == ability)
        .copied()
        .unwrap_or_else(|| AbilityOrder::manual(ability))
}

/// Replace a policy and keep persisted settings deterministic and compact.
pub fn set_ability_order(orders: &mut Vec<AbilityOrder>, order: AbilityOrder) {
    orders.retain(|previous| previous.ability != order.ability);
    if order.pass || order.answer.is_some() {
        orders.push(order);
        orders.sort_by_key(|order| order.ability);
    }
}

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

/// The five phases a turn is made of (CR 500.1), which is the grouping a bar
/// draws its twelve tiles in.
///
/// Twelve evenly-spaced tiles is a list; five groups is the turn. The steps
/// are not a flat sequence and never have been — three of them belong to the
/// beginning phase (CR 501.1), five to combat (CR 506.1), two to the ending
/// phase (CR 512.1), and the two main phases have **no** steps at all, which
/// is why they are single tiles here rather than an omission. A bar that
/// spreads twelve pills across a shelf says a turn has twelve equal parts.
/// It does not.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum RailPhase {
    /// Untap, upkeep and draw (CR 501.1).
    Beginning,
    /// The precombat main phase, which has no steps.
    PrecombatMain,
    /// Beginning of combat through end of combat (CR 506.1).
    Combat,
    /// The postcombat main phase, which has no steps.
    PostcombatMain,
    /// The end step and the cleanup step (CR 512.1).
    Ending,
}

/// The five phases, in turn order.
pub const RAIL_PHASES: [RailPhase; 5] = [
    RailPhase::Beginning,
    RailPhase::PrecombatMain,
    RailPhase::Combat,
    RailPhase::PostcombatMain,
    RailPhase::Ending,
];

impl RailPhase {
    /// The rows this phase is made of, in turn order.
    #[must_use]
    pub const fn rows(self) -> &'static [RailRow] {
        match self {
            Self::Beginning => &[RailRow::Untap, RailRow::Upkeep, RailRow::Draw],
            Self::PrecombatMain => &[RailRow::Main1],
            Self::Combat => &[
                RailRow::CombatBegin,
                RailRow::Attackers,
                RailRow::Blockers,
                RailRow::Damage,
                RailRow::CombatEnd,
            ],
            Self::PostcombatMain => &[RailRow::Main2],
            Self::Ending => &[RailRow::EndStep, RailRow::Cleanup],
        }
    }
}

impl RailRow {
    /// The phase this row belongs to (CR 500.1).
    #[must_use]
    pub const fn phase(self) -> RailPhase {
        match self {
            Self::Untap | Self::Upkeep | Self::Draw => RailPhase::Beginning,
            Self::Main1 => RailPhase::PrecombatMain,
            Self::CombatBegin
            | Self::Attackers
            | Self::Blockers
            | Self::Damage
            | Self::CombatEnd => RailPhase::Combat,
            Self::Main2 => RailPhase::PostcombatMain,
            Self::EndStep | Self::Cleanup => RailPhase::Ending,
        }
    }
}

impl RailRow {
    /// The rail label.
    #[must_use]
    pub const fn name(self) -> Phrase {
        match self {
            Self::Untap => Phrase::RailUntap,
            Self::Upkeep => Phrase::RailUpkeep,
            Self::Draw => Phrase::RailDraw,
            Self::Main1 => Phrase::RailMain1,
            Self::CombatBegin => Phrase::RailCombatBegin,
            Self::Attackers => Phrase::RailAttackers,
            Self::Blockers => Phrase::RailBlockers,
            Self::Damage => Phrase::RailDamage,
            Self::CombatEnd => Phrase::RailCombatEnd,
            Self::Main2 => Phrase::RailMain2,
            Self::EndStep => Phrase::RailEndStep,
            Self::Cleanup => Phrase::RailCleanup,
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

    /// Whether a player can ever *ask in advance* to be stopped in this step.
    ///
    /// False for untap and for cleanup, and a rail button in either is a
    /// stop that can never fire whichever way it is set — so both rows are
    /// dead: always skipped, not togglable, and not reachable by the pointer
    /// or the keyboard.
    ///
    /// Untap is the unconditional half: *"No player receives priority during
    /// the untap step, so no spells can be cast or resolve and no abilities
    /// can be activated or resolve"* (CR 502.4).
    ///
    /// Cleanup is the conditional half, and the reason this predicate is
    /// about a *standing order* rather than about the rules. Normally no
    /// player receives priority there (CR 514.3); the exception is CR 514.3a
    /// — if a state-based action is performed or an ability triggers during
    /// the step, the active player gets priority and another cleanup step
    /// follows. But that window exists only *because* something happened,
    /// and the engine asks for it when it does. A green button here would
    /// therefore be a stop nobody can arrange in advance: on the ordinary
    /// cleanup there is nothing to stop in, and on the exceptional one the
    /// question arrives whether the button was set or not. The cost of
    /// greying it is exactly one thing — a player who wanted to hold up an
    /// instant *speculatively*, in case a cleanup trigger opens the window,
    /// can no longer arm that in the rail.
    #[must_use]
    pub const fn grants_priority(self) -> bool {
        !matches!(self, Self::Untap | Self::Cleanup)
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
/// Fresh accounts stop at every priority window (#130). Quiet and competitive
/// presets are explicit opt-ins; upkeep and draw can contain meaningful plays.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

impl Default for PhaseOrders {
    fn default() -> Self {
        Self {
            skip: RailPreset::EveryStep.table(),
            selected: None,
        }
    }
}

impl PhaseOrders {
    /// Toggles a button between green (priority) and red (skip).
    ///
    /// A row the rules hand no priority in has nothing to toggle, and this
    /// is where that is enforced rather than in the drawing: a rail that
    /// merely *drew* the untap row as unclickable would still turn green
    /// under a keyboard, a preset, or a stored blob from a client that had
    /// the button. See [`RailRow::grants_priority`].
    pub fn toggle(&mut self, side: RailSide, row: RailRow) {
        if !row.grants_priority() {
            return;
        }
        let i = row.index();
        self.skip[side.index()][i] = !self.skip[side.index()][i];
    }

    /// Whether a button is red (skip).
    ///
    /// Always true for a step no player receives priority in, whatever the
    /// stored table says — there is no window there to stop in.
    #[must_use]
    pub fn is_skipped(&self, side: RailSide, row: RailRow) -> bool {
        !row.grants_priority() || self.skip[side.index()][row.index()]
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
    /// selection yet, starts at the first live row of your rail.
    ///
    /// Rows the rules grant no priority in are stepped straight over rather
    /// than landed on: they cannot be toggled, so resting the focus on one is
    /// a press that does nothing and reads as a broken key.
    pub fn move_selection(&mut self, delta: i32) {
        let span = (RAIL_ROWS.len() * 2) as i32;
        let at = |flat: i32| {
            let i = flat as usize;
            if flat < RAIL_ROWS.len() as i32 {
                (RailSide::Theirs, RAIL_ROWS[i])
            } else {
                (RailSide::Mine, RAIL_ROWS[i - RAIL_ROWS.len()])
            }
        };
        let Some((side, row)) = self.selected else {
            // No selection yet: start at the first live row of your rail.
            self.selected = RAIL_ROWS
                .into_iter()
                .find(|r| r.grants_priority())
                .map(|r| (RailSide::Mine, r));
            return;
        };
        let base = match side {
            RailSide::Theirs => 0,
            RailSide::Mine => RAIL_ROWS.len() as i32,
        };
        // Never zero, so the walk below always leaves where it started; a
        // `delta` that is a multiple of the rail's length is a request to go
        // exactly nowhere and would otherwise loop the whole way round.
        let step = if delta >= 0 { 1 } else { -1 };
        let mut flat = (base + row.index() as i32 + delta).rem_euclid(span);
        // Bounded by the rail's own length: one full lap and the search is
        // over, whatever the rows say.
        for _ in 0..span {
            if at(flat).1.grants_priority() {
                break;
            }
            flat = (flat + step).rem_euclid(span);
        }
        self.selected = Some(at(flat));
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

    /// Puts the whole rail to a preset, leaving the keyboard selection alone.
    pub fn set_to(&mut self, preset: RailPreset) {
        self.skip = preset.table();
    }

    /// Whether the rail is exactly this preset.
    ///
    /// Which is how the chips are drawn lit: a preset is a starting point, and
    /// the moment one button is toggled by hand the rail is the player's own
    /// again and no chip should go on claiming it.
    #[must_use]
    pub fn is(&self, preset: RailPreset) -> bool {
        self.skip == preset.table()
    }
}

/// A ready-made rail.
///
/// Three of them, because a preset with no way back is a trap: competitive
/// stops turn seventeen of the twenty-four buttons red, and clicking them
/// green again one at a time is not an undo. That argument is also why
/// [`Self::QuietSteps`] exists rather than the default living only in
/// `PhaseOrders::default`: [`PhaseOrders::is`] lights a chip on an exact table
/// match, so a default nothing can name draws a fresh account with no chip lit
/// and nothing to click to get back to it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RailPreset {
    /// Green everywhere. The safest rail there is, and slow.
    EveryStep,
    /// Green everywhere a decision is made, red in the five steps where one
    /// is not: untap, upkeep, draw, combat damage and cleanup. The default.
    QuietSteps,
    /// The windows a player used to a competitive client expects to be asked
    /// about, and no others.
    Competitive,
}

impl RailPreset {
    /// All three, in the order a settings screen should offer them — from the
    /// rail that stops most to the one that stops least.
    pub const ALL: [Self; 3] = [Self::EveryStep, Self::QuietSteps, Self::Competitive];

    /// The button's name.
    #[must_use]
    pub const fn label(self) -> Phrase {
        match self {
            Self::EveryStep => Phrase::RailPresetEveryStep,
            Self::QuietSteps => Phrase::RailPresetQuietSteps,
            Self::Competitive => Phrase::RailPresetCompetitive,
        }
    }

    /// The sentence under it, naming the stops rather than counting them.
    #[must_use]
    pub const fn detail(self) -> Phrase {
        match self {
            Self::EveryStep => Phrase::RailPresetEveryStepDetail,
            Self::QuietSteps => Phrase::RailPresetQuietStepsDetail,
            Self::Competitive => Phrase::RailPresetCompetitiveDetail,
        }
    }

    /// The preset as the `skip` table itself.
    fn table(self) -> [[bool; 12]; 2] {
        match self {
            Self::EveryStep => [[false; 12]; 2],
            Self::QuietSteps => {
                let mut skip = [[false; 12]; 2];
                for row in QUIET_ROWS {
                    for side in RailSide::BOTH {
                        skip[side.index()][row.index()] = true;
                    }
                }
                skip
            }
            Self::Competitive => {
                let mut skip = [[true; 12]; 2];
                for (side, row) in COMPETITIVE_STOPS {
                    skip[side.index()][row.index()] = false;
                }
                skip
            }
        }
    }
}

/// The five rows [`RailPreset::QuietSteps`] turns red, on both sides.
///
/// Untap and cleanup are on the list for completeness rather than for effect:
/// both are dead rows ([`RailRow::grants_priority`]) and read as red whatever
/// any preset writes. The other three are on it because the window is real
/// and empty: at upkeep and at draw nothing has changed since the end step
/// before, and combat damage is resolved before priority is handed back, so
/// the window after it is the one the end-of-combat row already covers.
///
/// What is deliberately *not* here: both main phases, both combat declaration
/// steps, end of combat and the end step. Those are where the game is played.
const QUIET_ROWS: [RailRow; 5] = [
    RailRow::Untap,
    RailRow::Upkeep,
    RailRow::Draw,
    RailRow::Damage,
    RailRow::Cleanup,
];

/// The seven windows [`RailPreset::Competitive`] keeps green.
///
/// Four of them are there because a player wants them: both of their own main
/// phases, and the end step of an opponent's turn, which is where an instant
/// goes.
///
/// The other three are there because **red is not "pass" in a combat
/// declaration step**. [`auto_answer`] turns a red row into
/// `DeclareNoAttackers` / `DeclareNoBlockers`, which is a decision and not a
/// skipped window, so the row on whichever side actually asks this seat to
/// declare has to stay green: attackers on its own turn, blockers on an
/// opponent's. A preset that got this wrong would not merely stop asking — it
/// would decline every block for the rest of the game.
///
/// `Attackers` on an opponent's turn is the one judgement call. It is not a
/// declaration this seat makes, and it is kept anyway: it is the window
/// between attackers and blocks, which is where removal goes, and skipping it
/// would be answering the very question the preset exists to leave open.
const COMPETITIVE_STOPS: [(RailSide, RailRow); 7] = [
    (RailSide::Mine, RailRow::Main1),
    (RailSide::Mine, RailRow::Attackers),
    (RailSide::Mine, RailRow::Blockers),
    (RailSide::Mine, RailRow::Main2),
    (RailSide::Theirs, RailRow::Attackers),
    (RailSide::Theirs, RailRow::Blockers),
    (RailSide::Theirs, RailRow::EndStep),
];

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
/// Bundled rather than passed as loose flags, because they are always read
/// together and swapping `mine` for `active_is_mine` is a bug no signature
/// would catch.
// That bundling is also the lint's own remedy: these are four independent
// yes/no facts about one moment, named where they are answered so a call
// site cannot hand them over in the wrong order. There is no state the game
// is *in* here — every one of the sixteen combinations is a real window.
#[allow(clippy::struct_excessive_bools)]
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
    /// Whether anything on the stack belongs to the other side.
    ///
    /// Answered by the caller and not here, because "the other side" is a
    /// question about the roster (teams) and this module knows only about
    /// turns. Its own spell resolving is what a player passing priority
    /// *wants*; an opponent's is the one thing a standing order must not
    /// answer for them.
    pub opposing_stack: bool,
    /// Whether this client is offering the seat something the engine's own
    /// list does not name.
    ///
    /// `LegalActions` is what the engine will accept *right now*, and right
    /// now is with the mana still in the lands: a hand of spells over four
    /// untapped Forests is an empty `castable`, an empty `abilities`, and a
    /// player with plenty to do. [`nothing_to_do`] read the engine's list
    /// alone, so `pass_when_nothing_to_do` passed the window out from under
    /// them — and every land they had not spent emptied at the end of the
    /// step.
    ///
    /// Answered by the caller because it is the shell that plans the taps:
    /// `crate::manaplan` needs the card registry to know what a printed mana
    /// ability makes, and this crate does not have it. It is the union of the
    /// two indigo sets — a spell whose lands could be tapped for it, and a
    /// card that could be suspended the same way.
    pub offering: bool,
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
        // Nothing to answer with is nothing to answer with, stack or no
        // stack. This rule fires only when the seat has no land, no spell, no
        // ability and nothing to suspend, so withholding the pass would leave
        // a player looking at a window whose only legal action is the one
        // being withheld.
        //
        // `at.offering` is the half the engine's list cannot answer: what
        // this client would tap lands *for*. Without it the rule fired on a
        // hand full of spells and a board full of untapped lands, which is
        // the commonest board there is.
        Pending::Priority { legal, .. }
            if rules.pass_when_nothing_to_do && !at.offering && nothing_to_do(legal) =>
        {
            AutoAnswer::Pass
        }
        // Every other automatic pass stops while the other side has something
        // on the stack. A red rail row means "I do nothing here when nothing
        // is happening"; it never meant "let their sorcery resolve
        // unanswered", and without this line it did, because `Situation`
        // carried no stack at all. The same goes for `skip_opponent_turns`
        // and for the autopilot, which is allowed to fast-forward to a
        // boundary and never to make a real decision.
        Pending::Priority { .. } if at.opposing_stack => AutoAnswer::None,
        Pending::Priority { .. } if skipped || quiet_turn || pilot.is_some() => AutoAnswer::Pass,
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

    /// The grouping and the list are two spellings of one turn.
    ///
    /// Written out twice on purpose — a `match` per row and a slice per phase
    /// — because the renderer walks the phases and everything else walks
    /// [`RAIL_ROWS`], and a row that fell out of the grouping would simply
    /// stop being drawn while every test about tiles still passed. Reading
    /// one out of the other would only ask whether it equalled itself.
    #[test]
    fn the_five_phases_are_the_twelve_steps_and_nothing_else() {
        let flat: Vec<RailRow> = RAIL_PHASES
            .iter()
            .flat_map(|phase| phase.rows().iter().copied())
            .collect();
        assert_eq!(
            flat,
            RAIL_ROWS.to_vec(),
            "the phases concatenated are the rail, in turn order"
        );
        for row in RAIL_ROWS {
            assert!(
                row.phase().rows().contains(&row),
                "{row:?} says it is in {:?}, which does not carry it",
                row.phase()
            );
        }
        // CR 501.1, CR 506.1 and CR 512.1 in one line; the two main phases
        // have no steps, which is the whole reason they are single tiles.
        let sizes: Vec<usize> = RAIL_PHASES.iter().map(|p| p.rows().len()).collect();
        assert_eq!(sizes, vec![3, 1, 5, 1, 2], "the shape of a turn");
    }

    fn at(mine: bool, active_is_mine: bool, phase: Phase, step: Step) -> Situation {
        Situation {
            mine,
            active_is_mine,
            phase,
            step,
            opposing_stack: false,
            offering: false,
        }
    }

    /// A priority window with something in it.
    ///
    /// One land to play, which makes it a window the player would actually be
    /// asked about — so a test using it is testing the rail, the autopilot or
    /// the rules it names, and not `pass_when_nothing_to_do`, which is on by
    /// default and would otherwise answer every one of them first.
    fn priority_pending() -> Pending {
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                can_pass: true,
                lands: vec![baylee_core::ids::ObjectId::new(9, 0)],
                castable: vec![],
                mana_abilities: vec![],
                abilities: vec![],
                suspendable: vec![],
            }),
        }
    }

    /// A priority window offering nothing at all.
    fn nothing_pending() -> Pending {
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
    fn a_default_client_still_asks_wherever_the_game_is_played() {
        // The default rail is no longer green everywhere — see
        // `the_default_rail_is_red_exactly_where_nothing_is_decided` for which
        // rows moved and why. What must not have changed is this: in a window
        // where the player has something to do, on a row where something is
        // decided, a fresh account is asked.
        let orders = PhaseOrders::default();
        for (active_is_mine, phase, step) in [
            (true, Phase::FirstMain, Step::Main),
            (true, Phase::SecondMain, Step::Main),
            (true, Phase::Ending, Step::End),
            (false, Phase::FirstMain, Step::Main),
            (false, Phase::Ending, Step::End),
        ] {
            assert_eq!(
                auto_answer(
                    &priority_pending(),
                    at(true, active_is_mine, phase, step),
                    &orders,
                    &AutoRules::default(),
                    None,
                ),
                AutoAnswer::None,
                "answered {phase:?}/{step:?} for its player"
            );
        }
    }

    /// The one thing a preset must never do.
    ///
    /// A red row is `DeclareNoAttackers` / `DeclareNoBlockers` in a
    /// declaration step, not a pass — so a preset that reds the row where
    /// *this* seat is the one declaring would not stop asking, it would
    /// answer, for the rest of the game. The test is written through
    /// `auto_answer` rather than against the table, because the table is not
    /// the claim: what a red row means there is.
    #[test]
    fn no_preset_ever_declares_for_its_player() {
        for preset in RailPreset::ALL {
            let mut orders = PhaseOrders::default();
            orders.set_to(preset);
            assert_eq!(
                auto_answer(
                    &Pending::ChooseAttackers {
                        player: PlayerId::new(0),
                        attackers: vec![],
                        defenders: Vec::new(),
                    },
                    at(true, true, Phase::Combat, Step::DeclareAttackers),
                    &orders,
                    &AutoRules {
                        skip_empty_attacks: false,
                        skip_empty_blocks: false,
                        ..AutoRules::default()
                    },
                    None,
                ),
                AutoAnswer::None,
                "{preset:?} declined an attack for its player"
            );
            assert_eq!(
                auto_answer(
                    &Pending::ChooseBlockers {
                        player: PlayerId::new(0),
                        blockers: vec![],
                        attacker: PlayerId::new(1),
                    },
                    at(true, false, Phase::Combat, Step::DeclareBlockers),
                    &orders,
                    &AutoRules {
                        skip_empty_attacks: false,
                        skip_empty_blocks: false,
                        ..AutoRules::default()
                    },
                    None,
                ),
                AutoAnswer::None,
                "{preset:?} declined a block for its player"
            );
        }
    }

    /// What competitive stops actually buy, and what they leave alone.
    #[test]
    fn competitive_stops_pass_the_quiet_windows_and_keep_the_loud_ones() {
        let mut orders = PhaseOrders::default();
        orders.set_to(RailPreset::Competitive);
        let answer = |active_is_mine, phase, step| {
            auto_answer(
                &priority_pending(),
                at(true, active_is_mine, phase, step),
                &orders,
                &AutoRules::default(),
                None,
            )
        };

        // Passed: an upkeep, a draw step, the end of your own turn.
        assert_eq!(
            answer(true, Phase::Beginning, Step::Upkeep),
            AutoAnswer::Pass
        );
        assert_eq!(answer(true, Phase::Beginning, Step::Draw), AutoAnswer::Pass);
        assert_eq!(answer(true, Phase::Ending, Step::End), AutoAnswer::Pass);
        assert_eq!(
            answer(false, Phase::SecondMain, Step::Main),
            AutoAnswer::Pass
        );

        // Kept: both your main phases, and theirs' end step — the two places
        // a player puts a spell.
        assert_eq!(answer(true, Phase::FirstMain, Step::Main), AutoAnswer::None);
        assert_eq!(
            answer(true, Phase::SecondMain, Step::Main),
            AutoAnswer::None
        );
        assert_eq!(answer(false, Phase::Ending, Step::End), AutoAnswer::None);

        // …and the whole of combat, on both turns: the declarations are
        // decisions and the windows around them are where a trick goes.
        for active_is_mine in [true, false] {
            for step in [Step::DeclareAttackers, Step::DeclareBlockers] {
                assert_eq!(
                    answer(active_is_mine, Phase::Combat, step),
                    AutoAnswer::None,
                    "{step:?} on {}",
                    if active_is_mine {
                        "your turn"
                    } else {
                        "theirs"
                    }
                );
            }
        }
    }

    /// One of the presets is the default, said twice — which is the whole
    /// reason the default has a preset at all: `is()` lights a chip on an
    /// exact table match, so a default no chip can name draws a fresh account
    /// with nothing lit and nothing to click to get back to.
    #[test]
    fn the_every_step_preset_is_what_a_fresh_account_already_has() {
        let mut orders = PhaseOrders::default();
        assert!(!orders.is(RailPreset::QuietSteps));
        assert!(orders.is(RailPreset::EveryStep));
        assert!(!orders.is(RailPreset::Competitive));

        orders.set_to(RailPreset::Competitive);
        assert!(orders.is(RailPreset::Competitive));
        // One button by hand and it is nobody's preset any more, which is what
        // stops a chip claiming a rail the player has since edited.
        orders.toggle(RailSide::Mine, RailRow::Upkeep);
        assert!(!orders.is(RailPreset::Competitive));
        assert!(!orders.is(RailPreset::EveryStep));
        assert!(!orders.is(RailPreset::QuietSteps));

        orders.set_to(RailPreset::EveryStep);
        assert!(orders.same_as(&PhaseOrders::default()));
    }

    #[test]
    fn the_default_rail_preserves_every_priority_window() {
        let orders = PhaseOrders::default();
        for side in RailSide::BOTH {
            for row in RAIL_ROWS {
                let quiet = !row.grants_priority();
                assert_eq!(
                    orders.is_skipped(side, row),
                    quiet,
                    "{side:?} {row:?} is on the wrong colour"
                );
            }
        }
        // Said the other way round, because this is the half that matters:
        // every row where a decision is actually made is still green, and the
        // two declaration rows are green because red is *not* pass there.
        for side in RailSide::BOTH {
            for row in [
                RailRow::Main1,
                RailRow::CombatBegin,
                RailRow::Attackers,
                RailRow::Blockers,
                RailRow::CombatEnd,
                RailRow::Main2,
                RailRow::EndStep,
            ] {
                assert!(!orders.is_skipped(side, row), "{side:?} {row:?} went red");
            }
        }
    }

    #[test]
    fn a_red_row_passes_and_stays_out_of_combat() {
        // From the all-green preset, so a toggle here means "turn this row
        // red" — three of the twelve rows start red now, and a test that
        // toggled one of those would be turning it green.
        let mut orders = PhaseOrders::default();
        orders.set_to(RailPreset::EveryStep);
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

    /// A red row means "I do nothing here when nothing is happening". It
    /// never meant "let their sorcery resolve unanswered", and it did:
    /// `Situation` carried no stack, so every automatic pass fired straight
    /// through an opponent's spell.
    #[test]
    fn nothing_automatic_passes_while_the_other_side_has_the_stack() {
        // From all-green, so the toggle below reddens a row rather than
        // greening one the new default already had red.
        let mut orders = PhaseOrders::default();
        orders.set_to(RailPreset::EveryStep);
        orders.toggle(RailSide::Mine, RailRow::Damage);
        let held = Situation {
            opposing_stack: true,
            ..at(true, true, Phase::Combat, Step::CombatDamage)
        };

        // The rail.
        assert_eq!(
            auto_answer(
                &priority_pending(),
                held,
                &orders,
                &AutoRules::default(),
                None
            ),
            AutoAnswer::None
        );
        // The autopilot, which may fast-forward and may not decide.
        assert_eq!(
            auto_answer(
                &priority_pending(),
                held,
                &PhaseOrders::default(),
                &AutoRules::default(),
                Some(&AutoPilot::ToNextPhase {
                    from: Phase::Combat
                }),
            ),
            AutoAnswer::None
        );
        // And skipping an opponent's turn.
        let theirs = Situation {
            opposing_stack: true,
            ..at(true, false, Phase::FirstMain, Step::Main)
        };
        let quiet = AutoRules {
            skip_opponent_turns: true,
            ..AutoRules::default()
        };
        assert_eq!(
            auto_answer(
                &priority_pending(),
                theirs,
                &PhaseOrders::default(),
                &quiet,
                None
            ),
            AutoAnswer::None
        );

        // The one rule that still fires, because it is the one that means
        // there is nothing to answer with: withholding the pass there would
        // leave a window whose only legal action is the one being withheld.
        let empty_handed = AutoRules {
            pass_when_nothing_to_do: true,
            ..AutoRules::default()
        };
        assert_eq!(
            auto_answer(
                &nothing_pending(),
                held,
                &PhaseOrders::default(),
                &empty_handed,
                None
            ),
            AutoAnswer::Pass
        );

        // …and with the stack this seat's own, the rail passes as before.
        assert_eq!(
            auto_answer(
                &priority_pending(),
                at(true, true, Phase::Combat, Step::CombatDamage),
                &orders,
                &AutoRules::default(),
                None
            ),
            AutoAnswer::Pass
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
                &AutoRules {
                    skip_empty_attacks: false,
                    skip_empty_blocks: false,
                    ..AutoRules::default()
                },
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
                &AutoRules {
                    skip_empty_attacks: false,
                    skip_empty_blocks: false,
                    ..AutoRules::default()
                },
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
                &AutoRules {
                    skip_empty_attacks: false,
                    skip_empty_blocks: false,
                    ..AutoRules::default()
                },
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
                &AutoRules {
                    skip_empty_attacks: false,
                    skip_empty_blocks: false,
                    ..AutoRules::default()
                },
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
                &AutoRules {
                    skip_empty_attacks: false,
                    skip_empty_blocks: false,
                    ..AutoRules::default()
                },
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
            Some((RailSide::Mine, RailRow::Upkeep)),
            "starts at your rail's first row that can be stopped in"
        );
        orders.move_selection(-1);
        assert_eq!(
            orders.selected(),
            Some((RailSide::Theirs, RailRow::EndStep)),
            "wraps upward into the opponent rail, over the dead cleanup and \
             untap rows"
        );
        orders.move_selection(1);
        assert_eq!(orders.selected(), Some((RailSide::Mine, RailRow::Upkeep)));
        orders.move_selection(4);
        assert_eq!(
            orders.selected(),
            Some((RailSide::Mine, RailRow::Attackers))
        );
        orders.clear_selection();
        assert_eq!(orders.selected(), None);
    }

    /// The untap and cleanup rows are dead, and they are dead in the *model*
    /// rather than in the drawing.
    ///
    /// No player receives priority during the untap step (CR 502.4), and in
    /// the cleanup step none does either unless something happened that made
    /// one (CR 514.3, 514.3a) — a window the engine opens on its own and
    /// which no button arranged in advance. So a rail button in either is a
    /// stop that can never fire whichever colour it is. A rail that only drew
    /// them unclickable would still turn one green under a preset, a stored
    /// blob from a client that had the button, or a keyboard walking onto it
    /// — and each of those is a green light that means nothing.
    #[test]
    fn the_steps_nobody_can_arrange_a_stop_in_cannot_be_switched_on() {
        let dead = [RailRow::Untap, RailRow::Cleanup];
        let mut orders = PhaseOrders::default();
        for side in RailSide::BOTH {
            for row in dead {
                assert!(orders.is_skipped(side, row));
                orders.toggle(side, row);
                assert!(
                    orders.is_skipped(side, row),
                    "{side:?} {row:?} took a toggle it has no window for"
                );
            }
        }
        // The all-green preset is the other way in, and it must not find one.
        orders.set_to(RailPreset::EveryStep);
        for side in RailSide::BOTH {
            for row in dead {
                assert!(orders.is_skipped(side, row));
            }
        }
        for row in dead {
            assert!(!row.grants_priority());
        }
        for row in RAIL_ROWS.into_iter().filter(|r| !dead.contains(r)) {
            assert!(row.grants_priority(), "{row:?} lost its priority window");
        }
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
        let empty = nothing_pending();
        // Switched off, an empty window is still the player's to pass.
        let mut rules = AutoRules {
            pass_when_nothing_to_do: false,
            ..AutoRules::default()
        };
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

    /// A hand this client could pay for is not passed away.
    ///
    /// `pass_when_nothing_to_do` is on by default and reads `LegalActions`,
    /// which is the engine's answer *with the mana still in the lands*: a
    /// hand of spells over four untapped Forests is an empty `castable`, an
    /// empty `abilities`, and a player with plenty to do. The engine's own
    /// list cannot say so, and the rule passed the window away — after which
    /// the pool empties at the end of the step and the turn's lands are gone.
    ///
    /// It is `Situation::offering` that carries the missing half, set by the
    /// shell from the two indigo sets. The negative case below is the whole
    /// point of the flag: with nothing to reach for, the rule still fires.
    #[test]
    fn a_window_this_client_is_offering_something_in_is_not_passed_away() {
        let rules = AutoRules {
            pass_when_nothing_to_do: true,
            ..AutoRules::default()
        };
        let mine = at(true, true, Phase::FirstMain, Step::Main);
        assert_eq!(
            auto_answer(
                &nothing_pending(),
                mine,
                &PhaseOrders::default(),
                &rules,
                None
            ),
            AutoAnswer::Pass,
            "nothing offered by anybody is still nothing to do"
        );
        assert_eq!(
            auto_answer(
                &nothing_pending(),
                Situation {
                    offering: true,
                    ..mine
                },
                &PhaseOrders::default(),
                &rules,
                None
            ),
            AutoAnswer::None,
            "but a spell the lands could be tapped for is something to do"
        );
    }
}

#[cfg(test)]
mod ability_order_tests {
    use super::*;
    use baylee_core::ids::{AbilityRef, CardIndex};
    use baylee_engine::choice::StandingAnswer;

    #[test]
    fn settings_keep_independent_answers_for_each_card_ability() {
        let a = AbilityRef::new(CardIndex::new(5), 0);
        let b = AbilityRef::new(CardIndex::new(5), 1);
        let mut prefs = crate::prefs::Preferences::default();
        set_ability_order(
            &mut prefs.ability_orders,
            AbilityOrder {
                ability: b,
                pass: false,
                answer: Some(StandingAnswer::No),
            },
        );
        set_ability_order(
            &mut prefs.ability_orders,
            AbilityOrder {
                ability: a,
                pass: true,
                answer: Some(StandingAnswer::Yes),
            },
        );
        let saved = crate::prefs::Preferences::from_json(&prefs.to_json());
        assert_eq!(saved.ability_orders, prefs.ability_orders);
        assert!(ability_order(&saved.ability_orders, a).pass);
        assert_eq!(
            ability_order(&saved.ability_orders, b).answer,
            Some(StandingAnswer::No)
        );
        set_ability_order(&mut prefs.ability_orders, AbilityOrder::manual(a));
        assert_eq!(
            prefs.ability_orders,
            vec![ability_order(&saved.ability_orders, b)]
        );
        assert!(
            crate::prefs::Preferences::from_json("{}")
                .ability_orders
                .is_empty()
        );
    }
}
