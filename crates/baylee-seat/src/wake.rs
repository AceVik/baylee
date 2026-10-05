//! The standing orders: a mind is woken only for real decisions.
//!
//! Most questions a seat is asked are priority rounds with nothing to do,
//! and waking a model for each would cost minutes a turn. The filter
//! answers those itself, with the client's own rule where the client has
//! one ([`automation::auto_answer`] on a [`PhaseOrders`] rail), and holds
//! the lines below whatever that rule says.
//!
//! # The rail
//!
//! [`MIND_STOPS`], the rail of the design (`llm-seat.md` §4.3): on its own
//! turn the seat is woken in both main phases; on the other side's turn once
//! their attackers are declared and at their end step. Every other window
//! passes, unless the other side has something on the stack.
//!
//! The rail answers priority and nothing else. The client's rail also
//! answers a declaration on a red row with "nobody attacks" or "nobody
//! blocks", which is how a person says "never stop here"; a mind has not
//! said that, so the filter declares for it only when there is nothing to
//! declare, and the rows can be red where the design has them red.
//!
//! # The lines under the rail
//!
//! [`WakeFilter::judge`] holds these itself rather than trusting a preset or
//! `auto_answer` to go on holding them:
//!
//! - only a priority round, an attack with no attacker and a block with no
//!   blocker are ever answered; every other question goes to the mind;
//! - no round is passed while a payment is owed, while the other side has
//!   something on the stack, or in a cleanup window, unless the seat has
//!   nothing at all it could do;
//! - while the seat's own payment is under way, nothing is answered for it:
//!   the next priority round with mana in the pool (passing would throw the
//!   mana away) and whatever the activation asks on the way (which colour a
//!   land makes) are [`Verdict::Continue`], the next step of what the mind
//!   began. A mind that planned the payment answers those from its plan
//!   (§4.3: a mana plan in flight wakes nobody), so they are counted apart
//!   from wakes.
//!
//! # What "nothing to do" means here
//!
//! Less than the engine's list says, and more. The engine offers every mana
//! ability at every priority (CR 605.3a), so a seat with one untapped land
//! never has "nothing but passing" by that list, and the client's rule
//! passes almost no window of the opponent's turn: in self-play that was a
//! fifth of all wakes, each one passed. Here an offer that can only make
//! mana ([`baylee_ai::only_makes_mana`]) is not something to do; a mana
//! ability that costs more than its tap (a sacrifice) still is.
//!
//! The engine's list counts only mana already floating, too, so a hand of
//! spells over untapped lands reads as nothing to do. The seat is `offering`
//! whatever its lands could pay for (the house's mana reader,
//! [`baylee_ai::mana_sources`], fed to the client's matcher), as the client
//! offers it. This reader lacks the client's target and cast-mode filters,
//! so it offers more than the client would: the error is always a wake too
//! many, never a window passed away.

use baylee_client_core::automation::{
    self, AutoAnswer, PhaseOrders, RAIL_ROWS, RailPreset, RailRow, RailSide, Situation,
};
use baylee_client_core::manaplan::Plan;
use baylee_client_core::prefs::AutoRules;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::ManaCost;
use baylee_core::types::TypeSet;
use baylee_engine::choice::{LegalActions, Pending, PlayerAction};
use baylee_view::{Phase, PlayerView, Step};

/// The windows a mind is woken in when it could do something there: its
/// own two main phases, and the other side's attack and end step.
pub const MIND_STOPS: [(RailSide, RailRow); 4] = [
    (RailSide::Mine, RailRow::Main1),
    (RailSide::Mine, RailRow::Main2),
    (RailSide::Theirs, RailRow::Attackers),
    (RailSide::Theirs, RailRow::EndStep),
];

/// What the standing orders make of one question.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The orders answer it; nobody is woken.
    Standing {
        /// The answer.
        action: PlayerAction,
        /// Which order gave it.
        why: Standing,
    },
    /// A real decision: the mind is woken, for this reason.
    Wake(Why),
    /// The next step of a payment the seat began: the mind is asked, marked
    /// as continuing, and no order may answer it.
    Continue,
}

/// Which standing order answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Standing {
    /// Nothing the seat could do but make mana, even by tapping.
    NothingToDo,
    /// A window the rail passes.
    QuietWindow,
    /// An attack with nothing that can attack.
    NoAttackers,
    /// A block with nothing that can block.
    NoBlockers,
}

/// Why the mind was woken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Why {
    /// A question the orders never answer: a mulligan, a target, a number,
    /// a card to discard.
    Question,
    /// An attack or a block with something in it to declare.
    Declaration,
    /// A window the rail stops at, with something the seat could do.
    RailStop,
    /// The other side has something on the stack.
    OpposingStack,
    /// A cleanup window, which opens only because something happened.
    Cleanup,
    /// A payment is owed.
    Owed,
    /// A question that is not this seat's, which only the mind may judge.
    NotAsked,
}

/// The standing orders of one seat.
#[derive(Clone, Debug)]
pub struct WakeFilter {
    orders: PhaseOrders,
    rules: AutoRules,
    /// Where the seat's last answer was an activation, while its pool holds
    /// mana: the moment the seat is paying for something.
    paying: Option<(u32, Phase, Step)>,
    hold_until_my_turn: bool,
}

impl Default for WakeFilter {
    fn default() -> Self {
        Self::new(&MIND_STOPS)
    }
}

impl WakeFilter {
    /// A filter that stops at `stops` and passes every other window, with
    /// the client's default rules otherwise.
    #[must_use]
    pub fn new(stops: &[(RailSide, RailRow)]) -> Self {
        let mut orders = PhaseOrders::default();
        orders.set_to(RailPreset::EveryStep);
        for side in RailSide::BOTH {
            for row in RAIL_ROWS {
                if row.grants_priority() && !stops.contains(&(side, row)) {
                    orders.toggle(side, row);
                }
            }
        }
        Self {
            orders,
            rules: AutoRules::default(),
            paying: None,
            hold_until_my_turn: false,
        }
    }

    /// Updates the filter with the model's chosen stops and hold instruction.
    pub fn apply_stops(&mut self, stops: Option<&crate::narrator::Stops>, hold: Option<&String>) {
        if let Some(stops) = stops {
            let mut orders = PhaseOrders::default();
            orders.set_to(RailPreset::EveryStep);
            for side in RailSide::BOTH {
                for row in RAIL_ROWS {
                    if !row.grants_priority() {
                        continue;
                    }
                    let keep = match side {
                        RailSide::Mine => stops
                            .mine
                            .iter()
                            .any(|s| s.eq_ignore_ascii_case(row_id(row))),
                        RailSide::Theirs => stops
                            .theirs
                            .iter()
                            .any(|s| s.eq_ignore_ascii_case(row_id(row))),
                    };
                    if !keep {
                        orders.toggle(side, row);
                    }
                }
            }
            self.orders = orders;
        }
        if let Some(hold) = hold
            && hold == "until_my_turn"
        {
            self.hold_until_my_turn = true;
        }
    }

    /// Whether the rail stops at this row: a test's window on the rail.
    #[must_use]
    pub fn stops_at(&self, side: RailSide, row: RailRow) -> bool {
        !self.orders.is_skipped(side, row)
    }

    /// What the orders make of `pending`, asked of this seat in `view`.
    ///
    /// `teams` is which side each seat plays for, in seat order (empty or
    /// `None` for a chair on its own side): "the other side" is a question
    /// about the roster, and a teammate's spell on the stack is not an
    /// opponent's.
    #[must_use]
    pub fn judge(&mut self, view: &PlayerView, pending: &Pending, teams: &[Option<u8>]) -> Verdict {
        if pending.asked() != Some(view.seat) {
            // Not this seat's question: nothing to answer, and nothing the
            // orders may answer for anybody else.
            return Verdict::Wake(Why::NotAsked);
        }
        if self.hold_until_my_turn && same_side(teams, view.active, view.seat) {
            self.hold_until_my_turn = false;
        }
        let paying = self.paying.take();
        let here = Some((view.turn, view.phase, view.step));
        match pending {
            Pending::Priority { .. } if paying == here && pool(view) > 0 => {
                self.paying = paying;
                Verdict::Continue
            }
            Pending::Priority { .. }
            | Pending::ChooseAttackers { .. }
            | Pending::ChooseBlockers { .. } => self.judge_window(view, pending, teams),
            // What the activation itself asks (the colour a dual land makes,
            // a number, a target) before the mana is in the pool: the mind
            // answers it, as the next step of what it began.
            _ if paying == here => {
                self.paying = paying;
                Verdict::Continue
            }
            _ => Verdict::Wake(Why::Question),
        }
    }

    fn judge_window(&self, view: &PlayerView, pending: &Pending, teams: &[Option<u8>]) -> Verdict {
        match pending {
            Pending::Priority { legal, .. } => self.priority(view, pending, legal, teams),
            Pending::ChooseAttackers { attackers, .. } if attackers.is_empty() => standing(
                PlayerAction::DeclareAttackers {
                    attackers: Vec::new(),
                },
                Standing::NoAttackers,
            ),
            Pending::ChooseBlockers { blockers, .. } if blockers.is_empty() => standing(
                PlayerAction::DeclareBlockers {
                    blockers: Vec::new(),
                },
                Standing::NoBlockers,
            ),
            Pending::ChooseAttackers { .. } | Pending::ChooseBlockers { .. } => {
                Verdict::Wake(Why::Declaration)
            }
            _ => Verdict::Wake(Why::Question),
        }
    }

    fn priority(
        &self,
        view: &PlayerView,
        pending: &Pending,
        legal: &LegalActions,
        teams: &[Option<u8>],
    ) -> Verdict {
        // A payment owed is answered with mana, which is all this seat may
        // have on offer then: never the "nothing to do" below.
        if view.owed.is_some() {
            return Verdict::Wake(Why::Owed);
        }
        let offering = offering(view, legal);
        if !offering && nothing_but_mana(view, legal) {
            return standing(PlayerAction::PassPriority, Standing::NothingToDo);
        }
        let opposing_stack = view
            .stack
            .iter()
            .any(|object| !same_side(teams, object.controller, view.seat));
        // Held here as well as in `auto_answer`: an opposing stack and a
        // cleanup window are decisions whatever the rail says.
        if opposing_stack {
            return Verdict::Wake(Why::OpposingStack);
        }
        if view.step == Step::Cleanup {
            return Verdict::Wake(Why::Cleanup);
        }
        if self.hold_until_my_turn {
            return standing(PlayerAction::PassPriority, Standing::QuietWindow);
        }
        let at = Situation {
            mine: true,
            active_is_mine: same_side(teams, view.active, view.seat),
            phase: view.phase,
            step: view.step,
            opposing_stack,
            offering,
            owing: view.owed.is_some(),
        };
        match automation::auto_answer(pending, at, &self.orders, &self.rules, None) {
            AutoAnswer::Pass => standing(PlayerAction::PassPriority, Standing::QuietWindow),
            _ => Verdict::Wake(Why::RailStop),
        }
    }

    /// Tells the filter what the seat answered, whoever answered it.
    ///
    /// An activation that leaves mana in the pool is a payment under way,
    /// and the next priority round in the same step is its next step.
    /// Read off the pool rather than off the ability: a mana ability puts
    /// nothing on the stack and leaves mana behind, and anything else that
    /// happens to leave the pool full errs toward asking.
    ///
    /// The questions an activation asks on its way (which colour a dual land
    /// makes, how much, at what) are part of it and leave the payment under
    /// way. Only an answer that ends one ends it: a pass, a cast, a land, a
    /// declaration. Listed that way round so that an answer this filter has
    /// not heard of keeps asking rather than passing a full pool away.
    pub fn heard(&mut self, view: &PlayerView, action: &PlayerAction) {
        match action {
            PlayerAction::ActivateManaAbility { .. } | PlayerAction::ActivateAbility { .. } => {
                self.paying = Some((view.turn, view.phase, view.step));
            }
            PlayerAction::PassPriority
            | PlayerAction::CastSpell { .. }
            | PlayerAction::PlayLand { .. }
            | PlayerAction::Suspend { .. }
            | PlayerAction::DeclareAttackers { .. }
            | PlayerAction::DeclareBlockers { .. }
            | PlayerAction::MulliganKeep
            | PlayerAction::MulliganTake
            | PlayerAction::Concede => self.paying = None,
            _ => {}
        }
    }
}

/// Whether the engine offers this seat nothing but passing and making mana
/// it has nothing to spend on.
fn nothing_but_mana(view: &PlayerView, legal: &LegalActions) -> bool {
    legal.lands.is_empty()
        && legal.castable.is_empty()
        && legal.suspendable.is_empty()
        && legal
            .abilities
            .iter()
            .all(|&(object, index)| baylee_ai::only_makes_mana(view, object, index))
}

fn standing(action: PlayerAction, why: Standing) -> Verdict {
    Verdict::Standing { action, why }
}

/// Whether two seats play for the same side (CR 102.3): the same seat, or
/// the same team at a table that has teams.
fn same_side(teams: &[Option<u8>], a: PlayerId, b: PlayerId) -> bool {
    if a == b {
        return true;
    }
    let side = |p: PlayerId| teams.get(usize::from(p.get())).copied().flatten();
    matches!((side(a), side(b)), (Some(x), Some(y)) if x == y)
}

/// The mana in the seat's pool.
fn pool(view: &PlayerView) -> u64 {
    view.seat(baylee_client_core::decision::resource_player(view))
        .map_or(0, |seat| seat.mana_pool.total())
}

/// One card the seat could cast by tapping what it has untapped, and the
/// taps that pay for it.
///
/// The one reader of "what could this seat cast from its lands": the
/// standing orders count these ([`offering`]) and the narrator lists them,
/// each with its taps, so a model is never woken to a menu that offers it
/// nothing but passing, and never shown a cast the orders would have passed
/// over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reach {
    /// The card, in hand or in the command zone.
    pub object: ObjectId,
    /// Its identity, as the view shows it.
    pub card: baylee_view::CardIdentity,
    /// Where it would be cast from.
    pub from: ReachFrom,
    /// The taps, in order, and the cost they pay: the printed cost with
    /// `{X}` at 0, and a commander's tax (CR 903.8) added.
    pub plan: Plan,
}

/// Where a [`Reach`] is cast from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReachFrom {
    /// The seat's hand.
    Hand,
    /// The command zone: a commander, with its tax.
    CommandZone,
}

/// Every card the seat could cast by tapping what it has untapped: a card
/// in hand the engine does not list yet (its mana is still in the lands)
/// whose timing allows it now, whose printed cost the seat's sources pay
/// and which is not a counterspell with nothing on the stack
/// ([`targets_the_stack`]), and a commander in the command zone with its
/// tax.
///
/// Cards the engine already lists as castable (the mana is floating) are
/// not here: they need no taps. Nor is a card whose cost the pool already
/// holds that the engine does not list: something other than mana stops
/// it.
#[must_use]
pub fn reachable(view: &PlayerView, legal: &LegalActions) -> Vec<Reach> {
    let sources = baylee_ai::mana_sources(view, legal);
    if sources.is_empty() {
        return Vec::new();
    }
    let Some(pool) = view
        .seat(baylee_client_core::decision::resource_player(view))
        .map(|seat| seat.mana_pool)
    else {
        return Vec::new();
    };
    // `{X}` at its least: the caster chooses it (CR 107.3a), the matcher
    // will not guess at it, and a spell castable with X = 0 is castable.
    // A plan with nothing to tap is no reach: the mana is in the pool
    // already, so what keeps the engine from listing the card is not mana
    // (a target, an additional cost, a raised cost), and no tap gives it.
    let payable = |cost: ManaCost| {
        cost.symbols().next()?;
        baylee_client_core::manaplan::plan(&cost.with_x(0), &pool, &sources)
            .filter(|plan| !plan.steps.is_empty())
    };
    let mut reach: Vec<Reach> = view
        .hand
        .iter()
        .filter(|card| !legal.castable.contains(&card.id) && !legal.lands.contains(&card.id))
        .filter(|card| !card.types.contains(TypeSet::LAND))
        .filter(|card| baylee_client_core::timing::allows(view, card.types, flash(card.card)))
        .filter(|card| !view.stack.is_empty() || !targets_the_stack(card.card))
        .filter_map(|card| {
            Some(Reach {
                object: card.id,
                card: card.card,
                from: ReachFrom::Hand,
                plan: payable(printed_cost(card.card)?)?,
            })
        })
        .collect();
    reach.extend(commanders(view, legal, &payable));
    reach
}

/// Whether the seat could cast something by tapping what it has untapped
/// ([`reachable`]).
#[must_use]
pub fn offering(view: &PlayerView, legal: &LegalActions) -> bool {
    !reachable(view, legal).is_empty()
}

/// Commanders standing in their owner's command zone, not yet castable,
/// whose cost with its tax (CR 903.8) the seat's sources pay.
fn commanders(
    view: &PlayerView,
    legal: &LegalActions,
    payable: &dyn Fn(ManaCost) -> Option<Plan>,
) -> Vec<Reach> {
    let Some(seat) = view.seat(baylee_client_core::decision::resource_player(view)) else {
        return Vec::new();
    };
    let zone = view.command.get(usize::from(view.seat.get()));
    seat.commanders
        .iter()
        .filter(|c| !legal.castable.contains(&c.object))
        .filter(|c| zone.is_some_and(|zone| zone.iter().any(|o| o.id == c.object)))
        .filter_map(|c| {
            let card = c.card?;
            let types = view.object(c.object).map_or(TypeSet::CREATURE, |o| o.types);
            if !baylee_client_core::timing::allows(view, types, flash(card))
                || view.stack.is_empty() && targets_the_stack(card)
            {
                return None;
            }
            let cost = printed_cost(card)?.with_more_generic(c.casts.saturating_mul(2));
            Some(Reach {
                object: c.object,
                card,
                from: ReachFrom::CommandZone,
                plan: payable(cost)?,
            })
        })
        .collect()
}

/// A card face's printed mana cost.
#[must_use]
pub fn printed_cost(card: baylee_view::CardIdentity) -> Option<ManaCost> {
    let def = baylee_cards::by_index(card.index)?;
    let face = def
        .faces
        .get(usize::from(card.face))
        .or(def.faces.first())?;
    Some(face.mana_cost)
}

/// Whether a card's spell must target a spell or an ability on the stack
/// (a counterspell's "target spell"). With the stack empty it has no legal
/// target, so it cannot be cast (CR 601.2c, 601.2e), whatever is tapped.
#[must_use]
pub fn targets_the_stack(card: baylee_view::CardIdentity) -> bool {
    use baylee_cards_dsl::{AbilityDef, TargetSpec};
    baylee_cards::by_index(card.index).is_some_and(|def| {
        def.abilities_for_face(usize::from(card.face))
            .iter()
            .any(|ability| {
                matches!(
                    ability,
                    AbilityDef::Spell { targets: Some(req), .. }
                        if req.min > 0
                            && matches!(
                                req.spec,
                                TargetSpec::Spell(_)
                                    | TargetSpec::AbilityOnStack(_)
                                    | TargetSpec::SpellOrAbility(_)
                            )
                )
            })
    })
}

/// Whether a card face has flash.
#[must_use]
pub fn flash(card: baylee_view::CardIdentity) -> bool {
    baylee_cards::by_index(card.index).is_some_and(|def| {
        def.keywords_for_face(usize::from(card.face))
            .contains(baylee_cards_dsl::KeywordSet::FLASH)
    })
}

#[cfg(test)]
mod tests;

fn row_id(row: RailRow) -> &'static str {
    match row {
        RailRow::Untap => "untap",
        RailRow::Upkeep => "upkeep",
        RailRow::Draw => "draw",
        RailRow::Main1 => "main1",
        RailRow::CombatBegin => "combat_begin",
        RailRow::Attackers => "attackers",
        RailRow::Blockers => "blockers",
        RailRow::Damage => "damage",
        RailRow::CombatEnd => "combat_end",
        RailRow::Main2 => "main2",
        RailRow::EndStep => "end_step",
        RailRow::Cleanup => "cleanup",
    }
}
