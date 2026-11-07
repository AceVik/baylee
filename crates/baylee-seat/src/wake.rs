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
//! [`MIND_STOPS`], the rail of the design (`docs/llm-protocol.md`
//! §"Stops"): on its own turn the seat is woken in both main phases; on the
//! other side's turn once their attackers are declared and at their end
//! step; and on either side once blockers are declared, in a combat it has
//! a creature in. Every other window passes, unless the other side has
//! something on the stack. A mind's `stops` replace the rows; the names it
//! may write are [`stop_row`]'s.
//!
//! The blockers row is held to a combat the seat fights in, whoever chose
//! it ([`fights`]): after blocks is the window for a pump, a regeneration
//! or a removal on a blocker (CR 509.4), and that is a decision only for a
//! seat with a creature in the combat. Without the row the seat passed
//! every such window, and games played by models had no combat tricks in
//! them; with it unheld, every combat would cost a call.
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
//! - the rail never passes a round in which the seat could activate
//!   something its card allows in this step alone (Ashen Ghoul's "activate
//!   only during your upkeep"): there is no other window for it;
//! - while a plan of the mind's runs, every question of the seat's is the
//!   plan's ([`Verdict::Planned`]): the plan's executor sees them all, and
//!   stops the plan at the first it cannot answer exactly;
//! - `until` passes the windows the rail would wake the mind in, and an
//!   opposing spell or ability as `react` says, and nothing else: never a
//!   question, a declaration with something to declare, a payment owed or a
//!   cleanup window. It ends at its boundary, at any wake, and at the
//!   mind's next answer without it (`docs/llm-protocol.md` §"Plans");
//! - while the seat's own payment is under way, nothing is answered for it:
//!   the next priority round with mana in the pool (passing would throw the
//!   mana away) and whatever the activation asks on the way (which colour a
//!   land makes) are [`Verdict::Continue`], the next step of what the mind
//!   began, and so is every round of the same step after a pass that let a
//!   trigger of the taps resolve (Manabarbs: [`WakeFilter::heard`]). A mind
//!   that planned the payment answers those from its plan (§4.3: a mana
//!   plan in flight wakes nobody), so they are counted apart from wakes.
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
//!
//! # A decision for a controlled player
//!
//! A seat that controls another player's turn (Mindslaver, CR 724.5) is
//! asked that player's questions, with that player's cards and mana
//! (`PlayerView::decision_player`, read through
//! [`decision::resource_player`]). Every rule here reads such a decision
//! from that player's chair: "nothing to do" is what that player's hand,
//! lands and pool could do, "mine" on the rail is that player's turn, "the
//! other side" is that player's opponents (this seat among them), and
//! `react: targets_me` means that player. The seat keeps a filter of its
//! own for those decisions (`SeatCore`), so an `until`, a plan's turn or a
//! payment under way of one player never answers the other's questions.

use baylee_client_core::automation::{
    self, AutoAnswer, PhaseOrders, RAIL_ROWS, RailPreset, RailRow, RailSide, Situation,
};
use baylee_client_core::decision;
use baylee_client_core::manaplan::Plan;
use baylee_client_core::prefs::AutoRules;
use baylee_core::ids::{CardIndex, ObjectId, PlayerId, TargetRef};
use baylee_core::mana::ManaCost;
use baylee_core::types::TypeSet;
use baylee_engine::choice::{CastModeKind, LegalActions, Pending, PlayerAction};
use baylee_view::{Phase, PlayerView, Step};

use crate::narrator::{React, Until};

mod convoke;
mod raised;
use convoke::{has_convoke, reduce_for_convoke};
pub use raised::Way;

/// The windows a mind is woken in when it could do something there: its
/// own two main phases, the other side's attack and end step, and on both
/// sides the window after blocks of a combat it fights in ([`fights`]).
pub const MIND_STOPS: [(RailSide, RailRow); 6] = [
    (RailSide::Mine, RailRow::Main1),
    (RailSide::Mine, RailRow::Blockers),
    (RailSide::Mine, RailRow::Main2),
    (RailSide::Theirs, RailRow::Attackers),
    (RailSide::Theirs, RailRow::Blockers),
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
    /// A question while a plan of the mind's runs: the mind is asked, and
    /// its plan answers it or stops there.
    Planned,
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
    /// The mind's `until`, while it holds.
    until: Option<Holding>,
    /// The turn a plan of the mind's runs in, while it has steps.
    planned: Option<u32>,
    /// How the last `until` ended, until the mind is told.
    held: Option<Held>,
}

/// An `until` the mind gave, while it holds.
#[derive(Clone, Copy, Debug)]
struct Holding {
    until: Until,
    react: React,
    /// The turn it was written in.
    turn: u32,
    /// Whether the seat has been somewhere other than its boundary since:
    /// `my_main2` written in the second main phase means the next one.
    armed: bool,
    /// Windows it passed that the seat would have been woken in.
    windows: u32,
}

/// What the mind asked the seat to go on doing after its answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Orders {
    /// How long to pass for it.
    pub until: Option<Until>,
    /// What an opposing spell or ability does meanwhile.
    pub react: React,
}

/// How an `until` ended, told in the mind's next message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held {
    /// What it was.
    pub until: Until,
    /// The windows it passed that would have woken the mind.
    pub windows: u32,
    /// Why it ended: `None` at its boundary.
    pub woke: Option<Why>,
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
            until: None,
            planned: None,
            held: None,
        }
    }

    /// A filter with this one's rail and rules and none of its state: no
    /// `until`, no plan, no payment under way. The filter a seat keeps for
    /// the decisions it makes for a player whose turn it controls (module
    /// docs), which stops where this seat's own mind said it stops.
    #[must_use]
    pub fn fresh(&self) -> Self {
        Self {
            orders: self.orders.clone(),
            rules: self.rules,
            paying: None,
            until: None,
            planned: None,
            held: None,
        }
    }

    /// Takes what the mind's answer asks of the filter: its stops, and, for
    /// an answer of the model's own (`orders`), its `until` and `react`;
    /// `planned` is how many steps of its plan are left after the answer,
    /// in `view`'s turn.
    pub fn apply_orders(
        &mut self,
        stops: Option<&crate::narrator::Stops>,
        orders: Option<Orders>,
        planned: usize,
        view: &PlayerView,
        teams: &[Option<u8>],
    ) {
        self.apply_stops(stops);
        self.planned = (planned > 0).then_some(view.turn);
        if let Some(orders) = orders {
            self.held = None;
            self.until = orders.until.map(|until| Holding {
                until,
                react: orders.react,
                turn: view.turn,
                armed: !boundary(until, view, view.turn, teams),
                windows: 0,
            });
        }
    }

    /// No plan of the mind's runs any more: it holds one plan at a time,
    /// and an answer for another player's question said none is left.
    pub const fn drop_plan(&mut self) {
        self.planned = None;
    }

    /// How the last `until` ended, once, for the mind's next message.
    pub const fn take_held(&mut self) -> Option<Held> {
        self.held.take()
    }

    /// Ends the `until`, keeping how for the mind.
    fn end_until(&mut self, woke: Option<Why>) {
        if let Some(holding) = self.until.take()
            && (holding.windows > 0 || woke.is_some())
        {
            self.held = Some(Held {
                until: holding.until,
                windows: holding.windows,
                woke,
            });
        }
    }

    /// Updates the filter with the model's chosen stops.
    pub fn apply_stops(&mut self, stops: Option<&crate::narrator::Stops>) {
        if let Some(stops) = stops {
            let mut orders = PhaseOrders::default();
            orders.set_to(RailPreset::EveryStep);
            for side in RailSide::BOTH {
                for row in RAIL_ROWS {
                    if !row.grants_priority() {
                        continue;
                    }
                    let named = |names: &[String]| names.iter().any(|s| stop_row(s) == Some(row));
                    let keep = match side {
                        RailSide::Mine => named(&stops.mine),
                        RailSide::Theirs => named(&stops.theirs),
                    };
                    if !keep {
                        orders.toggle(side, row);
                    }
                }
            }
            self.orders = orders;
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
    ///
    /// A question for a player whose turn this seat controls is this
    /// seat's to answer, and is judged from that player's chair (module
    /// docs).
    #[must_use]
    pub fn judge(&mut self, view: &PlayerView, pending: &Pending, teams: &[Option<u8>]) -> Verdict {
        if pending.asked() != Some(decision::resource_player(view)) {
            // Not this seat's question: nothing to answer, and nothing the
            // orders may answer for anybody else.
            return Verdict::Wake(Why::NotAsked);
        }
        // A plan runs in the turn it was written in, and sees every
        // question of it: no order answers one for it.
        if let Some(turn) = self.planned {
            if turn == view.turn {
                return Verdict::Planned;
            }
            self.planned = None;
        }
        if let Some(holding) = &mut self.until {
            if boundary(holding.until, view, holding.turn, teams) {
                if holding.armed {
                    self.end_until(None);
                }
            } else {
                holding.armed = true;
            }
        }
        let verdict = self.judge_orders(view, pending, teams);
        if let Verdict::Wake(why) = verdict {
            self.end_until(Some(why));
        }
        verdict
    }

    fn judge_orders(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> Verdict {
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

    fn judge_window(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> Verdict {
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
        &mut self,
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
        let me = decision::resource_player(view);
        let opposing_stack = view
            .stack
            .iter()
            .any(|object| decision::opposes(view, object, me, |a, b| same_side(teams, a, b)));
        // Held here as well as in `auto_answer`: an opposing stack and a
        // cleanup window are decisions whatever the rail says, unless the
        // mind's `react` says otherwise of the stack.
        if opposing_stack {
            if let Some(holding) = &mut self.until {
                let passes = match holding.react {
                    React::All => false,
                    React::TargetsMe => !targets_me(view, teams),
                    React::None => true,
                };
                if passes {
                    holding.windows += 1;
                    return standing(PlayerAction::PassPriority, Standing::QuietWindow);
                }
            }
            return Verdict::Wake(Why::OpposingStack);
        }
        if view.step == Step::Cleanup {
            return Verdict::Wake(Why::Cleanup);
        }
        // After blocks, a trick is a decision only in a combat the seat
        // fights in (module docs, "The rail").
        if view.step == Step::DeclareBlockers && !fights(view, teams) {
            return standing(PlayerAction::PassPriority, Standing::QuietWindow);
        }
        let at = Situation {
            mine: true,
            active_is_mine: same_side(teams, view.active, me),
            phase: view.phase,
            step: view.step,
            opposing_stack,
            offering,
            owing: view.owed.is_some(),
        };
        if automation::auto_answer(pending, at, &self.orders, &self.rules, None) == AutoAnswer::Pass
            && !bound_to_this_step(view, legal)
        {
            return standing(PlayerAction::PassPriority, Standing::QuietWindow);
        }
        // A window the rail stops at: the mind's `until` passes it.
        if let Some(holding) = &mut self.until {
            holding.windows += 1;
            return standing(PlayerAction::PassPriority, Standing::QuietWindow);
        }
        Verdict::Wake(Why::RailStop)
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
    ///
    /// A pass over abilities alone does not end it either. A land's own
    /// trigger (Manabarbs, City of Brass, Psychic Venom) goes on the stack
    /// as the land is tapped (CR 603.3), and a sorcery-speed spell cannot be
    /// cast until it has resolved, so the seat's taps pass to let it
    /// ([`only_abilities_on_stack`]). Priority comes back in the same step
    /// with the mana still in the pool (CR 117.3b, 500.5), and that window
    /// is the payment's next step, the cast it was for: ending the payment
    /// at the pass handed those windows to the orders, which passed them
    /// as nothing to do, and the mana emptied with the step.
    pub fn heard(&mut self, view: &PlayerView, action: &PlayerAction) {
        match action {
            PlayerAction::ActivateManaAbility { .. } | PlayerAction::ActivateAbility { .. } => {
                self.paying = Some((view.turn, view.phase, view.step));
            }
            PlayerAction::PassPriority
                if self.paying.is_some() && only_abilities_on_stack(view) => {}
            PlayerAction::PassPriority
            | PlayerAction::CastSpell { .. }
            | PlayerAction::PlayLand { .. }
            | PlayerAction::Suspend { .. }
            | PlayerAction::Foretell { .. }
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
        && legal.foretellable.is_empty()
        && reachable_abilities(view, legal).is_empty()
        && reachable_foretell(view, legal).is_empty()
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

/// Whether `view` is where `until`, written in `turn`, ends: the seat's
/// turn, its second main phase, its end step, or any turn after `turn`
/// (the turn of the player decided for, in a controlled decision).
fn boundary(until: Until, view: &PlayerView, turn: u32, teams: &[Option<u8>]) -> bool {
    let mine = same_side(teams, view.active, decision::resource_player(view));
    match until {
        Until::MyTurn => mine,
        Until::MyMain2 => mine && view.phase == Phase::SecondMain,
        Until::MyEnd => mine && view.step == Step::End,
        Until::EndOfTurn => view.turn != turn,
    }
}

/// Whether something this seat answers ([`decision::opposes`]) targets it, a
/// permanent it controls, or its commander (`react: targets_me`), or in a
/// controlled decision the player decided for. Read from the targets the
/// view states, never from what a card is.
fn targets_me(view: &PlayerView, teams: &[Option<u8>]) -> bool {
    let me = decision::resource_player(view);
    let commanders: Vec<ObjectId> = view
        .seat(me)
        .map(|seat| seat.commanders.iter().map(|c| c.object).collect())
        .unwrap_or_default();
    view.stack
        .iter()
        .filter(|object| decision::opposes(view, object, me, |a, b| same_side(teams, a, b)))
        .flat_map(|object| &object.targets)
        .any(|target| match target {
            TargetRef::Player(player) => *player == me,
            TargetRef::Object(source) => {
                commanders.contains(&source.object)
                    || view
                        .object(source.object)
                        .is_some_and(|object| object.controller == me)
            }
        })
}

/// Whether the stack holds something, and nothing but abilities: what the
/// seat's taps pass over toward the spell they pay for, a land's trigger
/// set off by the taps themselves. A spell on the stack is a decision, and
/// never passed for a payment.
#[must_use]
pub fn only_abilities_on_stack(view: &PlayerView) -> bool {
    !view.stack.is_empty()
        && view
            .stack
            .iter()
            .all(|o| matches!(o.stack_item, Some(baylee_view::StackItem::Ability { .. })))
}

/// Whether the table is holding a payment window open (CR 605.3a,
/// `PlayerView::owed`): inside one the engine offers mana abilities and
/// the pass that settles it, and nothing else, so no spell or activation
/// is within reach by tapping. Taking one there taps lands the payment
/// needed, and the pass the taps end on leaves the payment unpaid.
#[must_use]
pub fn owes(view: &PlayerView) -> bool {
    view.owed.is_some()
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
    /// Cast face down for {3} with its morph or disguise (CR 702.37c,
    /// 702.168a) rather than for its printed cost.
    pub face_down: bool,
    /// The way it is cast at a price of its own (`wake/raised.rs`): entwined,
    /// kicked, replicated, overloaded. `None` for the printed cost.
    pub way: Option<Way>,
    /// The taps, in order, and the cost they pay: the printed cost with
    /// `{X}` at 0, a commander's tax (CR 903.8) added, and priced as the
    /// engine charges it, its increases and reductions applied
    /// (`baylee_ai::spell_price`, CR 601.2f).
    pub plan: Plan,
}

/// Where a [`Reach`] is cast from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReachFrom {
    /// The seat's hand.
    Hand,
    /// The command zone: a commander, with its tax.
    CommandZone,
    /// A graveyard: flashback or a graveyard casting permission.
    Graveyard,
    /// Exile: an exile casting permission.
    Exile,
}

/// One activated ability the seat could activate by tapping what it has
/// untapped, and the taps that pay for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbilityReach {
    /// The object whose ability is activated.
    pub source: ObjectId,
    /// The ability's index in `legal.unpaid_abilities`.
    pub ability_index: u32,
    /// The card identity, if known.
    pub card: Option<CardIndex>,
    /// The taps, in order, and the cost they pay.
    pub plan: Plan,
}

/// Every card the seat could cast by tapping what it has untapped: a card
/// in hand the engine does not list yet (its mana is still in the lands)
/// whose timing allows it now, whose price the seat's sources pay, which
/// the engine would cast once paid (`LegalActions::castable_once_paid`:
/// a legal target for each it must have, CR 601.2c) and which is not a
/// counterspell with nothing on the stack ([`targets_the_stack`]), and a
/// commander in the command zone with its tax.
///
/// Cards the engine already lists as castable (the mana is floating) are
/// not here: they need no taps. Nor is a card whose cost the pool already
/// holds that the engine does not list: something other than mana stops
/// it. Nor is anything while a payment is owed ([`owes`]).
///
/// The hand, the lands, the pool and the command zone are those of the
/// player whose cards the decision uses ([`decision::resource_player`]):
/// in a decision for a player whose turn this seat controls (CR 724.5),
/// that player's, never the seat's own.
#[must_use]
pub fn reachable(view: &PlayerView, legal: &LegalActions) -> Vec<Reach> {
    if owes(view) {
        return Vec::new();
    }
    let sources = baylee_ai::mana_sources(view, legal);
    if sources.is_empty() {
        return Vec::new();
    }
    let me = decision::resource_player(view);
    let hand = decision::hand(view);
    let Some(pool) = view.seat(me).map(|seat| seat.mana_pool) else {
        return Vec::new();
    };
    let convoke_creatures = convoke_creatures(view, me, &sources);
    // `{X}` at its least: the caster chooses it (CR 107.3a), the matcher
    // will not guess at it, and a spell castable with X = 0 is castable.
    // A plan with nothing to tap is no reach: the mana is in the pool
    // already, so what keeps the engine from listing the card is not mana
    // (a target, an additional cost, a raised cost), and no tap gives it.
    let whole_price = |id: ObjectId, card: baylee_view::CardIdentity, mut total: ManaCost| {
        if has_convoke(card) && !convoke_creatures.is_empty() {
            total = reduce_for_convoke(total, &convoke_creatures);
        }
        baylee_ai::plan_for_spell(&total, &pool, view, legal, id)
            .filter(|plan| !plan.steps.is_empty())
    };
    let payable =
        |id: ObjectId, card: baylee_view::CardIdentity, cost: ManaCost, mode: CastModeKind| {
            cost.symbols().next()?;
            // The price the engine charges, its reductions included (CR 601.2f).
            let total = baylee_ai::spell_price(view, id, Some(card), legal, mode, cost.with_x(0));
            whole_price(id, card, baylee_ai::less_delve(view, id, card, total))
        };
    let mut reach: Vec<Reach> = hand
        .iter()
        .filter(|card| {
            !legal.castable.contains(&card.id)
                || printed_cost(card.card).is_some_and(|c| {
                    let price = baylee_ai::spell_price(
                        view,
                        card.id,
                        Some(card.card),
                        legal,
                        CastModeKind::Normal,
                        c.with_x(0),
                    );
                    baylee_client_core::manaplan::plan(&price, &pool, &[]).is_none()
                })
        })
        .filter(|card| !card.types.contains(TypeSet::LAND))
        .filter(|card| baylee_client_core::timing::allows(view, card.types, flash(card.card)))
        .filter(|card| !view.stack.is_empty() || !targets_the_stack(card.card))
        .filter(|card| baylee_ai::spell_can_be_cast_now(view, legal, card.id, card.card))
        .filter_map(|card| {
            Some(Reach {
                object: card.id,
                card: card.card,
                from: ReachFrom::Hand,
                face_down: false,
                way: None,
                plan: payable(
                    card.id,
                    card.card,
                    printed_cost(card.card)?,
                    CastModeKind::Normal,
                )?,
            })
        })
        .collect();
    // Face down for {3} (CR 702.37c, 702.168a): a morph or disguise card is
    // a 2/2 creature spell that way, whatever it costs face up, so it is
    // reached with a creature's timing and the {3}, priced as the engine
    // prices that mode. Bug hunt of 3.11.2026, q44: Exalted Angel was never
    // offered with three lands, its printed {4}{W}{W} out of reach.
    reach.extend(
        hand.iter()
            .filter(|card| {
                !legal.castable.contains(&card.id) && face_down_keyword(card.card).is_some()
            })
            .filter(|_| baylee_client_core::timing::allows(view, TypeSet::CREATURE, false))
            .filter(|card| baylee_ai::spell_can_be_cast_now(view, legal, card.id, card.card))
            .filter_map(|card| {
                Some(Reach {
                    object: card.id,
                    card: card.card,
                    from: ReachFrom::Hand,
                    face_down: true,
                    way: None,
                    plan: payable(
                        card.id,
                        card.card,
                        ManaCost::parse("{3}"),
                        CastModeKind::Disguise,
                    )?,
                })
            }),
    );
    reach.extend(commanders(view, legal, &payable));
    reach.extend(graveyard_and_exile(view, legal, &payable));
    reach.retain(|plain| has_convoke(plain.card) || raised::stands(legal, plain));
    let raised = raised::ways(view, legal, &reach, &whole_price);
    reach.extend(raised);
    reach
}

/// The colours of the creatures that could convoke and are no mana source,
/// fewest colours first.
fn convoke_creatures(
    view: &PlayerView,
    me: PlayerId,
    sources: &[baylee_client_core::manaplan::Source],
) -> Vec<baylee_core::color::ColorSet> {
    let mut creatures: Vec<baylee_core::color::ColorSet> = view
        .battlefield
        .iter()
        .filter(|o| {
            o.controller == me
                && o.types.contains(TypeSet::CREATURE)
                && !o.status.is_tapped()
                && !o.status.is_phased_out()
                && !sources.iter().any(|s| s.id == o.id)
        })
        .map(|o| o.colors)
        .collect();
    creatures.sort_by_key(|c| c.len());
    creatures
}

/// Every activated ability in `legal.unpaid_abilities` whose mana cost the
/// seat's untapped sources can pay now; none while a payment is owed
/// ([`owes`]).
#[must_use]
pub fn reachable_abilities(view: &PlayerView, legal: &LegalActions) -> Vec<AbilityReach> {
    if owes(view)
        || legal.unpaid_abilities.is_empty()
        || baylee_ai::mana_sources(view, legal).is_empty()
    {
        return Vec::new();
    }
    let Some(pool) = view
        .seat(baylee_client_core::decision::resource_player(view))
        .map(|seat| seat.mana_pool)
    else {
        return Vec::new();
    };
    legal
        .unpaid_abilities
        .iter()
        .filter_map(|&(source, ability_index, cost)| {
            cost.symbols().next()?;
            let taps_self = ability_taps_self(view, source, ability_index);
            let plan = baylee_ai::plan_for_activation(
                &cost.with_x(0),
                &pool,
                view,
                legal,
                source,
                taps_self,
            )
            .filter(|plan| !plan.steps.is_empty())?;
            let card = view
                .object(source)
                .and_then(|o| o.card)
                .or_else(|| {
                    decision::known_cards(view)
                        .find(|c| c.id == source)
                        .map(|c| c.card)
                })
                .map(|c| c.index);
            Some(AbilityReach {
                source,
                ability_index,
                card,
                plan,
            })
        })
        .collect()
}

/// Every card in `legal.unpaid_foretell` whose {2} the seat's untapped
/// sources pay now, with the taps; none while a payment is owed
/// ([`owes`]). Foretell is a special action, not a spell (CR 702.143a), so
/// its {2} is planned as an activation's is: mana that may be spent only on
/// spells does not pay it.
#[must_use]
pub fn reachable_foretell(view: &PlayerView, legal: &LegalActions) -> Vec<(ObjectId, Plan)> {
    if owes(view)
        || legal.unpaid_foretell.is_empty()
        || baylee_ai::mana_sources(view, legal).is_empty()
    {
        return Vec::new();
    }
    let Some(pool) = view
        .seat(baylee_client_core::decision::resource_player(view))
        .map(|seat| seat.mana_pool)
    else {
        return Vec::new();
    };
    let cost = ManaCost::parse("{2}");
    legal
        .unpaid_foretell
        .iter()
        .filter_map(|&card| {
            let plan = baylee_ai::plan_for_activation(&cost, &pool, view, legal, card, false)
                .filter(|plan| !plan.steps.is_empty())?;
            Some((card, plan))
        })
        .collect()
}

/// The printed ability `index` of `source`, on the battlefield, in a
/// graveyard or in the hand, as the face it shows now prints it; `None`
/// for a special action, a granted ability or an object the view cannot
/// name.
fn printed_ability(
    view: &PlayerView,
    source: ObjectId,
    index: u32,
) -> Option<&'static baylee_cards_dsl::AbilityDef> {
    if baylee_engine::choice::is_special_action(index) {
        return None;
    }
    let rules = view
        .object(source)
        .and_then(|o| o.rules.or_else(|| o.card.map(Into::into)))
        .or_else(|| {
            decision::known_cards(view)
                .find(|c| c.id == source)
                .map(|c| baylee_view::RulesFace::from(c.card))
        })?;
    baylee_cards::by_index(rules.card)?
        .abilities_for_face(usize::from(rules.face))
        .get(usize::try_from(index).ok()?)
}

/// Whether the ability's cost taps `source`, whose mana then pays none of it.
/// A granted ability's cost is on no card it has, so the view says.
fn ability_taps_self(view: &PlayerView, source: ObjectId, index: u32) -> bool {
    use baylee_cards_dsl::AbilityDef;
    if let Some(slot) = baylee_engine::choice::granted_slot(index) {
        return view
            .object(source)
            .and_then(|object| object.grant(slot))
            .is_some_and(|grant| grant.taps);
    }
    match printed_ability(view, source, index) {
        Some(
            AbilityDef::Activated { cost, .. } | AbilityDef::ActivatedConditional { cost, .. },
        ) => cost.parts.contains(&baylee_cards_dsl::CostPart::TapSelf),
        _ => false,
    }
}

/// Whether the seat could activate something now that its card allows in
/// this step alone ("activate only during your upkeep": Ashen Ghoul's
/// return from the graveyard). Such a window is the ability's only one, so
/// the rail does not pass it whatever its rows say: on the default rail,
/// which has no upkeep row, the seat passed every upkeep the Ghoul could
/// have come back in (codex attempt 191, q215).
fn bound_to_this_step(view: &PlayerView, legal: &LegalActions) -> bool {
    use baylee_cards_dsl::{AbilityDef, Condition};
    fn during_a_step(condition: &Condition) -> bool {
        match condition {
            Condition::DuringStep(_) => true,
            Condition::All(all) => all.iter().any(during_a_step),
            _ => false,
        }
    }
    legal
        .abilities
        .iter()
        .copied()
        .filter(|&(source, index)| !baylee_ai::only_makes_mana(view, source, index))
        .chain(
            reachable_abilities(view, legal)
                .into_iter()
                .map(|reach| (reach.source, reach.ability_index)),
        )
        .any(|(source, index)| {
            matches!(
                printed_ability(view, source, index),
                Some(AbilityDef::ActivatedConditional { condition, .. }) if during_a_step(condition)
            )
        })
}

/// Whether the seat could cast, activate or foretell something by tapping
/// what it has untapped ([`reachable`], [`reachable_abilities`],
/// [`reachable_foretell`]).
#[must_use]
pub fn offering(view: &PlayerView, legal: &LegalActions) -> bool {
    !reachable(view, legal).is_empty()
        || !reachable_abilities(view, legal).is_empty()
        || !reachable_foretell(view, legal).is_empty()
}

/// Commanders standing in their owner's command zone, not yet castable,
/// whose cost with its tax (CR 903.8) the seat's sources pay.
fn commanders(
    view: &PlayerView,
    legal: &LegalActions,
    payable: &dyn Fn(ObjectId, baylee_view::CardIdentity, ManaCost, CastModeKind) -> Option<Plan>,
) -> Vec<Reach> {
    let me = decision::resource_player(view);
    let Some(seat) = view.seat(me) else {
        return Vec::new();
    };
    let zone = view.command.get(usize::from(me.get()));
    seat.commanders
        .iter()
        .filter(|c| !legal.castable.contains(&c.object))
        .filter(|c| zone.is_some_and(|zone| zone.iter().any(|o| o.id == c.object)))
        .filter_map(|c| {
            let card = c.card?;
            let types = view.object(c.object).map_or(TypeSet::CREATURE, |o| o.types);
            if !baylee_client_core::timing::allows(view, types, flash(card))
                || view.stack.is_empty() && targets_the_stack(card)
                || !baylee_ai::spell_can_be_cast_now(view, legal, c.object, card)
            {
                return None;
            }
            let cost = printed_cost(card)?.with_more_generic(c.casts.saturating_mul(2));
            Some(Reach {
                object: c.object,
                card,
                from: ReachFrom::CommandZone,
                face_down: false,
                way: None,
                plan: payable(c.object, card, cost, CastModeKind::Normal)?,
            })
        })
        .collect()
}

fn graveyard_and_exile(
    view: &PlayerView,
    legal: &LegalActions,
    payable: &dyn Fn(ObjectId, baylee_view::CardIdentity, ManaCost, CastModeKind) -> Option<Plan>,
) -> Vec<Reach> {
    let me = decision::resource_player(view);
    let mut out = Vec::new();
    for object in view
        .graveyards
        .iter()
        .flatten()
        .filter(|o| !legal.castable.contains(&o.id) && !o.types.contains(TypeSet::LAND))
    {
        let Some(card) = object.card else {
            continue;
        };
        if !baylee_client_core::timing::allows(view, object.types, flash(card))
            || view.stack.is_empty() && targets_the_stack(card)
            || !baylee_ai::spell_can_be_cast_now(view, legal, object.id, card)
        {
            continue;
        }
        let (Some(cost), mode) = (if let Some(fb) = object.flashback {
            (Some(fb), CastModeKind::Flashback)
        } else if object.owner == me && legal.payable.contains(&object.id) {
            (printed_cost(card), CastModeKind::Normal)
        } else {
            (None, CastModeKind::Normal)
        }) else {
            continue;
        };
        if let Some(plan) = payable(object.id, card, cost, mode) {
            out.push(Reach {
                object: object.id,
                card,
                from: ReachFrom::Graveyard,
                face_down: false,
                way: None,
                plan,
            });
        }
    }
    for object in view.exile.iter().flatten().filter(|o| {
        !legal.castable.contains(&o.id)
            && !o.types.contains(TypeSet::LAND)
            && legal.payable.contains(&o.id)
    }) {
        let Some(card) = object.card else {
            continue;
        };
        if !baylee_client_core::timing::allows(view, object.types, flash(card))
            || view.stack.is_empty() && targets_the_stack(card)
            || !baylee_ai::spell_can_be_cast_now(view, legal, object.id, card)
        {
            continue;
        }
        let Some(cost) = printed_cost(card) else {
            continue;
        };
        if let Some(plan) = payable(object.id, card, cost, CastModeKind::Normal) {
            out.push(Reach {
                object: object.id,
                card,
                from: ReachFrom::Exile,
                face_down: false,
                way: None,
                plan,
            });
        }
    }
    out
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

/// The keyword a card face is cast face down with for {3}: "morph" or
/// "disguise" (CR 702.37c, 702.168a), `None` for a card with neither. The
/// engine has one mode for both; a card with both is told "disguise".
#[must_use]
pub fn face_down_keyword(card: baylee_view::CardIdentity) -> Option<&'static str> {
    let face = baylee_cards::by_index(card.index)?
        .faces
        .get(usize::from(card.face))?;
    if face.disguise.is_some() {
        Some("disguise")
    } else if face.morph.is_some() {
        Some("morph")
    } else {
        None
    }
}

/// Whether a card face has flash.
#[must_use]
pub fn flash(card: baylee_view::CardIdentity) -> bool {
    baylee_cards::by_index(card.index).is_some_and(|def| {
        def.keywords_for_face(usize::from(card.face))
            .contains(baylee_cards_dsl::KeywordSet::FLASH)
    })
}

/// Whether a creature of the seat's side (in a controlled decision, the
/// side of the player decided for) attacks or blocks in this combat
/// (CR 506.4): the combat the blockers row stops for.
fn fights(view: &PlayerView, teams: &[Option<u8>]) -> bool {
    let me = decision::resource_player(view);
    let ours = |id: ObjectId| {
        view.object(id)
            .is_some_and(|object| same_side(teams, object.controller, me))
    };
    view.combat.attackers.iter().any(|a| ours(a.creature))
        || view.combat.blockers.iter().any(|b| ours(b.blocker))
}

/// The rail row a step's name in `stops` means: the name the header gives
/// it ([`crate::narrator::step_id`]: `main1`, `blockers`, `end_step`), or
/// the step's name in the rules (`declare_blockers`, `combat_damage`), in
/// any case, with spaces or hyphens for the underscores and with or
/// without `_step`. `None` for a name that is no step.
///
/// Models wrote `declare_blockers` and were never woken there: the rows
/// were matched by their short names alone (codex attempt 62).
#[must_use]
pub fn stop_row(name: &str) -> Option<RailRow> {
    let name = name.trim().to_ascii_lowercase().replace([' ', '-'], "_");
    let name = name.strip_suffix("_step").unwrap_or(&name);
    Some(match name {
        "untap" => RailRow::Untap,
        "upkeep" => RailRow::Upkeep,
        "draw" => RailRow::Draw,
        "main1" | "main_1" | "first_main" | "precombat_main" => RailRow::Main1,
        "combat_begin" | "begin_combat" | "beginning_combat" | "beginning_of_combat" => {
            RailRow::CombatBegin
        }
        "attackers" | "declare_attackers" => RailRow::Attackers,
        "blockers" | "declare_blockers" => RailRow::Blockers,
        "damage" | "combat_damage" | "first_strike_damage" => RailRow::Damage,
        "combat_end" | "end_combat" | "end_of_combat" => RailRow::CombatEnd,
        "main2" | "main_2" | "second_main" | "postcombat_main" => RailRow::Main2,
        "end" | "end_of_turn" => RailRow::EndStep,
        "cleanup" => RailRow::Cleanup,
        _ => return None,
    })
}

/// The short name of each step a model may stop at, as the header writes
/// it and the system prompt lists it.
#[must_use]
pub fn stop_id(row: RailRow) -> &'static str {
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

#[cfg(test)]
pub(crate) mod tests;
