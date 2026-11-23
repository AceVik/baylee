//! The ways a card in hand could be cast, and what each would cost to reach.
//!
//! # Why this exists
//!
//! The engine counts a spell's ways against the mana that is **floating**
//! (`cast_wizard::cast_options`, whose `afford` closure reads the seat's own
//! pool), which is the right question for the moment it asks it: the engine
//! has no planner and cannot know which lands are about to be tapped. The
//! client does have one, and it used to float **exactly the printed cost**
//! and then send `CastSpell` — so by the time the engine counted, only one
//! way was payable and there was nothing left to ask.
//!
//! That silence went both directions. An alternative cost **dearer** than the
//! printed one could never be chosen by clicking at all — Reveillark's evoke
//! is `{5}{W}` against a printed `{4}{W}`, and seven open mana cast it the
//! cheap way without a word. A **free** alternative was chosen just as
//! quietly in the other direction: Solitude with an empty pool is already in
//! `LegalActions::castable`, so the click cast it for its evoke and the
//! printed `{3}{W}{W}` was unreachable for a player who would rather keep the
//! white card in their hand.
//!
//! So the question has to come **before** the floating, and this module is
//! what it is asked from: every way this client can both *read* and *pay
//! for*, each with the plan that pays for it. The engine stays exactly as it
//! is.
//!
//! # What it refuses, and why that is the honest half
//!
//! A way offered here that the engine will not offer is a row that lies: the
//! player picks it, the mana is floated, and the answer the client has
//! remembered fits nothing in the engine's list. So a way is offered only
//! when **every** part of its cost is one this reader can evaluate from the
//! view:
//!
//! - the mana part, through [`baylee_client_core::manaplan`] as everywhere
//!   else — which is also where `{X}`, `{S}` and restricted mana are refused;
//! - [`CostPart::PayLife`], against the seat's own life total;
//! - [`CostPart::ExileFromHand`] with a colour filter, against the hand — the
//!   pitch on Force of Will, Force of Negation, Misdirection and Solitude;
//! - [`CostPart::Sacrifice`], [`CostPart::ReturnToHand`] and [`CostPart::TapN`],
//!   against distinct permanents the seat controls whose projected filters are
//!   publicly known.
//!
//! Anything else — another pitch-filter shape, a discard, a tap of one
//! separately chosen permanent — takes the way off the list rather than being
//! guessed at.
//!
//! # And what it does not reach
//!
//! From hand this reads normal, alternative, prototype and disguise costs,
//! legal land faces, and the modes of a "choose one" spell one of whose
//! modes has a cost of its own (overload: Cyclonic Rift's `{1}{U}` against
//! its overloaded `{6}{U}`), each mode a way at its own price. A nonland
//! face other than the front (an Adventure, the other side of a modal
//! double-faced card) needs the game state to be legal, and the engine
//! states it: such a way is read off `LegalActions::cast_ways`, at the whole
//! price named there. Other modal spells stay with the engine's own chooser
//! because their legality needs the game state. Miracle is not a choice a
//! hand click makes: the engine asks it as the card is drawn, and its cost is
//! paid in a payment window (`Duel::confirm_payment`).
//!
//! Nothing here reads convoke, delve or a printed cost reduction either —
//! the same three [`crate::mana_for`] and [`crate::reachable`] already do not
//! read. A way whose real price is lower than this thinks is a way that stays
//! off the list, which costs a click and never a wrong cast.
//!
//! The commander tax looks like a fourth of those and is not. Both of them
//! *do* read it (`with_more_generic(2 * commander.casts)`), because both also
//! offer to tap for a commander standing in the **command zone**, which is
//! the only place CR 903.8 charges it. This module reads a card in hand and
//! nothing else, so here there is no tax to add rather than one left unread.

use baylee_cards_dsl::{AltCondition, CostPart, Filter};
use baylee_client_core::decision::resource_player;
use baylee_client_core::manaplan::Plan;
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaCost;
use baylee_engine::choice::{CastModeKind, LegalActions};
use baylee_view::PlayerView;

/// One way to cast a card, and the taps that would pay for it.
#[derive(Clone, Debug)]
pub struct ReachableMode {
    /// Which way this is.
    ///
    /// The **kind** and never the engine's option index: that index is a
    /// position in a list the engine builds against the pool at the instant
    /// it asks, and the pool at that instant is the one this client is about
    /// to change.
    pub kind: CastModeKind,
    /// The mana part to float for it.
    pub cost: ManaCost,
    /// The taps that float it — empty when the mana is already up, which is
    /// also what a free alternative cost plans to.
    pub plan: Plan,
}

/// Every way `card` could be cast right now that this client can read and pay
/// for, in the order a chooser should draw them.
///
/// Printed cost first, then each alternative in the order the card prints
/// them, because that is the order the engine offers and the order the card
/// reads.
///
/// An empty list is an ordinary answer and so is a list of one: the caller
/// asks a question only when there is more than one way, and everything else
/// takes the path it always took.
#[must_use]
#[allow(clippy::too_many_lines)] // one branch per printed way to cast
pub fn reachable_modes(
    view: &PlayerView,
    legal: &LegalActions,
    card: ObjectId,
) -> Vec<ReachableMode> {
    // A library top is cast only through the permission the engine names.
    // In particular, Gwenom replaces the mana cost rather than adding an
    // ordinary hand-style cast, so never price its printed alternatives here.
    if let Some(top) = view
        .library_tops
        .iter()
        .find(|top| top.id == card && top.owner == resource_player(view))
    {
        return library_top_modes(view, legal, card, top.card);
    }
    let Some(hand) = baylee_client_core::decision::known_cards(view).find(|c| c.id == card) else {
        return Vec::new();
    };
    let Some(def) = baylee_cards::by_index(hand.card.index) else {
        return Vec::new();
    };
    // The same face `manasources::hand_cost` reads, for the same reason: an
    // adventure or an MDFC in hand is shown by whichever face is up, and a
    // cost read off the other one is a price nobody is being quoted.
    let Some(face) = def
        .faces
        .get(hand.card.face as usize)
        .or_else(|| def.faces.first())
    else {
        return Vec::new();
    };
    let Some((pool, life)) = view
        .seat(resource_player(view))
        .map(|s| (s.mana_pool, s.life))
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut offer = |kind, cost: ManaCost| {
        let cost = baylee_ai::spell_price(view, card, Some(hand.card), legal, kind, cost);
        if let Some(plan) = crate::manasources::plan_with_life_for_spell(
            &cost, &pool, view, legal, card, kind, life,
        ) {
            out.push(ReachableMode { kind, cost, plan });
        }
    };
    let face_index = usize::from(hand.card.face);
    let modal_only = baylee_engine::casting::modes_are_the_only_way(def, face_index);

    // CR 202.1b: a face with no printed cost has no printed way to be cast,
    // which is the rule `casting::has_a_printed_cost` states engine-side and
    // the one that keeps a suspend-only card off this list.
    if !modal_only
        && face.mana_cost.symbols().next().is_some()
        && (face.kicked_targets.is_none()
            || !crate::targeting::ordinary_targetless(view, hand.card))
    {
        let printed_flash = def
            .keywords_for_face(face_index)
            .contains(baylee_cards_dsl::KeywordSet::FLASH);
        let ordinary_timing = baylee_client_core::timing::allows(view, hand.types, printed_flash);
        if ordinary_timing || !face.optional_cost_grants_flash {
            offer(CastModeKind::Normal, face.mana_cost);
        }
        if let Some(opt_cost) = face.optional_cost
            && (ordinary_timing
                || face.optional_cost_grants_flash
                    && baylee_client_core::timing::allows(view, hand.types, true))
            && parts_payable(view, card, opt_cost.parts)
        {
            offer(
                CastModeKind::OptionalCost,
                face.mana_cost.combine(&opt_cost.mana),
            );
        }
    }

    // "Choose one" with a mode that costs something else (overload, CR
    // 702.96a): each mode is a way with its own price, so the choice comes
    // before the floating like any alternative cost. A mode whose targets
    // this board provably lacks is left off, as the kicker is above.
    offer_modal_override_modes(view, def, face_index, face, &mut offer);
    if let Some(req) = face.kicked_targets
        && crate::targeting::requirement_possible(view, req)
        && face
            .additional_costs
            .iter()
            .all(|c| parts_payable(view, card, c.parts))
    {
        offer(
            CastModeKind::Kicked,
            baylee_engine::casting::kicked_mana_cost(face),
        );
    }
    if let Some(prototype) = face.prototype {
        offer(CastModeKind::Prototype, prototype.cost);
    }
    if face.has_face_down_cast() {
        offer(CastModeKind::Disguise, const { ManaCost::parse("{3}") });
    }
    if let Some(bestow) = face.bestow
        && crate::targeting::requirement_possible(view, baylee_engine::casting::BESTOW_TARGET_REQ)
        && parts_payable(view, card, bestow.parts)
    {
        offer(CastModeKind::Bestow, bestow.mana);
    }
    if let Some(dash) = face.dash {
        offer(CastModeKind::Dash, dash);
    }
    if let Some(blitz) = face.blitz
        && parts_payable(view, card, blitz.parts)
    {
        offer(CastModeKind::Blitz, blitz.mana);
    }
    if let Some(warp) = face.warp
        && parts_payable(view, card, warp.parts)
    {
        offer(CastModeKind::Warp, warp.mana);
    }
    if let Some(sneak) = def.abilities_for_face(0).iter().find_map(|a| match a {
        baylee_cards_dsl::AbilityDef::Static(sa) => match sa.modifier {
            baylee_cards_dsl::Modifier::Sneak(cost) => Some(cost),
            _ => None,
        },
        _ => None,
    }) && (legal
        .cast_ways
        .iter()
        .any(|(c, w)| *c == card && w.kind == CastModeKind::Sneak)
        || (view.active == resource_player(view)
            && view.step == baylee_view::Step::DeclareBlockers
            && parts_payable(view, card, sneak.parts)
            && view.combat.attackers.iter().any(|a| {
                !a.blocked
                    && view
                        .object(a.creature)
                        .is_some_and(|o| o.controller == resource_player(view))
            })))
    {
        offer(CastModeKind::Sneak, sneak.mana);
    }
    for ability in def.abilities_for_face(face_index) {
        if let baylee_cards_dsl::AbilityDef::Awaken { cost, .. } = ability
            && crate::targeting::requirement_possible(
                view,
                baylee_cards_dsl::TargetReq::one(baylee_cards_dsl::TargetSpec::Object(
                    &baylee_cards_dsl::Filter::YOUR_LAND,
                )),
            )
            && parts_payable(view, card, cost.parts)
        {
            offer(CastModeKind::Awaken, cost.mana);
        }
    }
    if let Some(mutate) = face.mutate
        && crate::targeting::requirement_possible(view, baylee_engine::casting::MUTATE_TARGET_REQ)
    {
        offer(CastModeKind::Mutate, mutate);
    }
    if legal.lands.contains(&card) {
        for (index, face) in def.faces.iter().enumerate() {
            if face.types.contains(baylee_core::types::TypeSet::LAND) {
                offer(CastModeKind::PlayLandFace(index), ManaCost::ZERO);
            }
        }
    }
    for (i, alt) in face.alternative_costs.iter().enumerate() {
        if alternative_mode_open(view, legal, card, i, alt)
            && parts_payable(view, card, alt.cost.parts)
        {
            offer(CastModeKind::Alternative(i), alt.cost.mana);
        }
    }
    if let Some(emerge) = face.emerge
        && let Some(emerge_mode) = emerge_mode(view, legal, card, hand.card, emerge, &pool, life)
    {
        out.push(emerge_mode);
    }
    out.extend(face_modes(view, legal, card, &pool, life));
    out.extend(foretell_mode(view, legal, card));
    out
}

fn alternative_mode_open(
    view: &PlayerView,
    legal: &LegalActions,
    card: ObjectId,
    index: usize,
    alt: &baylee_cards_dsl::AlternativeCost,
) -> bool {
    condition_holds(view, alt.condition)
        || legal
            .cast_ways
            .iter()
            .any(|(c, w)| *c == card && w.kind == CastModeKind::Alternative(index))
}

fn emerge_mode(
    view: &PlayerView,
    legal: &LegalActions,
    card: ObjectId,
    identity: baylee_view::CardIdentity,
    emerge: baylee_cards_dsl::Emerge,
    pool: &baylee_view::ManaPoolView,
    life: i32,
) -> Option<ReachableMode> {
    let cost = if let Some((_, way)) = legal
        .cast_ways
        .iter()
        .find(|(id, w)| *id == card && w.kind == CastModeKind::Emerge)
    {
        way.cost
    } else if parts_payable(view, card, &[CostPart::Sacrifice(emerge.from)]) {
        let player = resource_player(view);
        let max_mv = view
            .battlefield_of(player)
            .filter(|o| {
                o.id != card && crate::targeting::matches(view, o, emerge.from) == Some(true)
            })
            .map(|o| o.mana_value)
            .max()
            .unwrap_or(0);
        baylee_ai::spell_price(
            view,
            card,
            Some(identity),
            legal,
            CastModeKind::Emerge,
            emerge.cost,
        )
        .with_less_generic(max_mv)
    } else {
        return None;
    };
    crate::manasources::plan_with_life_for_spell(
        &crate::reach::least_x(cost),
        pool,
        view,
        legal,
        card,
        CastModeKind::Emerge,
        life,
    )
    .map(|plan| ReachableMode {
        kind: CastModeKind::Emerge,
        cost,
        plan,
    })
}

fn offer_modal_override_modes(
    view: &PlayerView,
    def: &baylee_cards_dsl::CardDef,
    face_index: usize,
    face: &baylee_cards_dsl::FaceDef,
    offer: &mut impl FnMut(CastModeKind, ManaCost),
) {
    let Some((modes, choose, can_repeat)) = def
        .abilities_for_face(face_index)
        .iter()
        .find_map(baylee_cards_dsl::AbilityDef::modal_spell)
    else {
        return;
    };
    if can_repeat && !choose.is_one() {
        for packed in baylee_engine::casting::repeated_mode_sets(modes, choose) {
            if baylee_engine::casting::chosen_repeated_modes(modes, packed).all(|(_, mode)| {
                mode.targets
                    .is_none_or(|req| crate::targeting::requirement_possible(view, req))
                    && mode
                        .second_targets
                        .is_none_or(|req| crate::targeting::requirement_possible(view, req))
            }) {
                offer(
                    CastModeKind::RepeatedModes(packed),
                    baylee_engine::casting::repeated_mode_set_cost(face.mana_cost, modes, packed),
                );
            }
        }
        return;
    }
    if !choose.is_one() {
        for set in baylee_engine::casting::mode_sets(modes, choose) {
            if baylee_engine::casting::chosen_modes(modes, set).all(|(_, mode)| {
                mode.targets
                    .is_none_or(|req| crate::targeting::requirement_possible(view, req))
                    && mode
                        .second_targets
                        .is_none_or(|req| crate::targeting::requirement_possible(view, req))
            }) {
                offer(
                    CastModeKind::Modes(set),
                    baylee_engine::casting::mode_set_cost(face.mana_cost, modes, set),
                );
            }
        }
        return;
    }
    if modes.iter().any(|m| m.cost_override.is_some()) {
        for (i, mode) in modes.iter().enumerate() {
            if mode
                .targets
                .is_none_or(|req| crate::targeting::requirement_possible(view, req))
                && mode
                    .second_targets
                    .is_none_or(|req| crate::targeting::requirement_possible(view, req))
            {
                offer(
                    CastModeKind::Mode(i),
                    mode.cost_override.unwrap_or(face.mana_cost),
                );
            }
        }
    }
}

/// Price only the casting permission named for this visible library top.
fn library_top_modes(
    view: &PlayerView,
    legal: &LegalActions,
    card: ObjectId,
    identity: Option<baylee_view::CardIdentity>,
) -> Vec<ReachableMode> {
    let Some((pool, life)) = view
        .seat(resource_player(view))
        .map(|seat| (seat.mana_pool, seat.life))
    else {
        return Vec::new();
    };
    let mut out: Vec<_> = legal
        .cast_ways
        .iter()
        .filter(|(id, way)| *id == card && matches!(way.kind, CastModeKind::LibraryTopLife { .. }))
        .filter_map(|(_, way)| {
            crate::manasources::plan_with_life_for_spell(
                &crate::reach::least_x(way.cost),
                &pool,
                view,
                legal,
                card,
                way.kind,
                life,
            )
            .map(|plan| ReachableMode {
                kind: way.kind,
                cost: way.cost,
                plan,
            })
        })
        .collect();
    // A land still uses the ordinary land drop, including a land face beside
    // a spell face. The engine's land offer is the permission, not this reader.
    if legal.lands.contains(&card)
        && let Some(def) = identity.and_then(|card| baylee_cards::by_index(card.index))
    {
        for (index, face) in def.faces.iter().enumerate() {
            let kind = CastModeKind::PlayLandFace(index);
            if face.types.contains(baylee_core::types::TypeSet::LAND)
                && let Some(plan) = crate::manasources::plan_with_life_for_spell(
                    &ManaCost::ZERO,
                    &pool,
                    view,
                    legal,
                    card,
                    kind,
                    life,
                )
            {
                out.push(ReachableMode {
                    kind,
                    cost: ManaCost::ZERO,
                    plan,
                });
            }
        }
    }
    out
}

/// A face other than the front, cast from the hand: an Adventure (CR
/// 716.3), the other side of a modal double-faced card (CR 712.8b).
///
/// Whether that face may be cast now is a question about it alone (CR
/// 601.3e: its timing, its targets, a lock that names it), so the engine
/// answers it and names the whole price (`LegalActions::cast_ways`), and the
/// way is offered as named. Brazen Borrower beside two Islands was dark: its
/// own `{1}{U}{U}` is out of reach, and Petty Theft's `{1}{U}` was never
/// asked about. What restricted mana may pay for it is the engine's word on
/// that face too (`Spend::cast`): an Ancient Ziggurat's creature-only mana
/// pays for the Borrower and never for Petty Theft, an instant.
fn face_modes(
    view: &PlayerView,
    legal: &LegalActions,
    card: ObjectId,
    pool: &baylee_view::ManaPoolView,
    life: i32,
) -> Vec<ReachableMode> {
    legal
        .cast_ways
        .iter()
        .filter(|(id, way)| {
            *id == card
                && matches!(way.kind, CastModeKind::Face(_) | CastModeKind::Fused)
                && way.cost.symbols().next().is_some()
        })
        .filter_map(|(_, way)| {
            crate::manasources::plan_with_life_for_spell(
                &crate::reach::least_x(way.cost),
                pool,
                view,
                legal,
                card,
                way.kind,
                life,
            )
            .map(|plan| ReachableMode {
                kind: way.kind,
                cost: way.cost,
                plan,
            })
        })
        .collect()
}

/// The cost of an offered suspend action that can be funded now.
pub(crate) fn suspend_cost(
    duel: &crate::Duel,
    view: &PlayerView,
    legal: &LegalActions,
    card: ObjectId,
) -> Option<ManaCost> {
    if legal.suspendable.contains(&card) {
        let hand = baylee_client_core::decision::hand(view)
            .iter()
            .find(|c| c.id == card)?;
        let def = baylee_cards::by_index(hand.card.index)?;
        return def
            .abilities_for_face(0)
            .iter()
            .find_map(|ability| match ability {
                baylee_cards_dsl::AbilityDef::Suspend { cost, .. } => Some(*cost),
                _ => None,
            });
    }
    crate::suspend_mana_for(duel, card)?;
    legal
        .unpaid_suspend
        .iter()
        .find_map(|&(id, cost)| (id == card).then_some(cost))
}

/// The engine's paid or reachable foretell special action (CR 702.143a).
pub(crate) fn foretell_mode(
    view: &PlayerView,
    legal: &LegalActions,
    card: ObjectId,
) -> Option<ReachableMode> {
    let cost = const { ManaCost::parse("{2}") };
    let plan = if legal.foretellable.contains(&card) {
        Plan::default()
    } else if legal.unpaid_foretell.contains(&card) {
        let pool = view.seat(resource_player(view))?.mana_pool;
        baylee_client_core::manaplan::plan(&cost, &pool, &crate::manasources::sources(view, legal))?
    } else {
        return None;
    };
    Some(ReachableMode {
        kind: CastModeKind::Foretell,
        cost,
        plan,
    })
}

/// Whether an alternative cost's condition holds, read off the view.
///
/// Anything the view cannot establish is refused, keeping an unverified way
/// off the pre-tap list the way [`parts_payable`] refuses unreadable parts.
fn condition_holds(view: &PlayerView, condition: AltCondition) -> bool {
    match condition {
        AltCondition::Always => true,
        AltCondition::NotYourTurn => view.active != resource_player(view),
        // `casting::controls_a_commander`, in the words a view has: a
        // commander handle follows its card through every zone (CR 400.7), so
        // the battlefield is asked rather than the handle believed.
        AltCondition::CommanderControlled => view.seat(resource_player(view)).is_some_and(|seat| {
            seat.commanders.iter().any(|c| {
                view.battlefield
                    .iter()
                    .any(|o| o.id == c.object && o.controller == seat.player)
            })
        }),
        AltCondition::If(condition) => match condition {
            baylee_cards_dsl::Condition::ControlCount(filter, min)
                if !uses_self_identity(filter) =>
            {
                let payer = resource_player(view);
                view.battlefield
                    .iter()
                    .filter(|object| {
                        object.controller == payer
                            && !object.status.is_phased_out()
                            && crate::targeting::matches(view, object, filter) == Some(true)
                    })
                    .count()
                    >= usize::from(*min)
            }
            // "If an opponent controls an Island" (the Masques Legates, Sivvi's
            // Ruse): `eval::conditions` asks each opponent in turn whether
            // that one controls `min`. Exact at a duel and refused above it,
            // for the reason `targeting::matches` gives for
            // `ControlledByOpponent`: the view does not know the teams.
            baylee_cards_dsl::Condition::OpponentControlCount(filter, min)
                if view.seats.len() == 2 && !uses_self_identity(filter) =>
            {
                let payer = resource_player(view);
                view.battlefield
                    .iter()
                    .filter(|object| {
                        object.controller != payer
                            && !object.status.is_phased_out()
                            && crate::targeting::matches(view, object, filter) == Some(true)
                    })
                    .count()
                    >= usize::from(*min)
            }
            // "If an opponent controls a Swamp and you control a Plains": every
            // clause, each read as above, so one unreadable clause refuses all.
            baylee_cards_dsl::Condition::All(clauses) => clauses
                .iter()
                .all(|clause| condition_holds(view, AltCondition::If(clause))),
            baylee_cards_dsl::Condition::AmountCompare { left, op, right } => {
                match (view_amount(view, **left), view_amount(view, **right)) {
                    (Some(l), Some(r)) => op.holds(l, r),
                    _ => false,
                }
            }
            // Conditions whose inputs the view does not carry (graveyard
            // history, per-turn tallies, source counters) stay refused rather
            // than guessed at.
            _ => false,
        },
    }
}

/// Evaluates an `Amount` when the `PlayerView` holds its exact value; returns
/// `None` for any quantity the view cannot prove (so pre-tap never guesses).
fn view_amount(view: &PlayerView, amount: baylee_cards_dsl::Amount) -> Option<u32> {
    use baylee_cards_dsl::{Amount, PlayerRel, ZoneSel};
    let payer = resource_player(view);
    match amount {
        Amount::Fixed(n) => Some(n),
        Amount::HandSize(PlayerRel::You) => view.seat(payer).map(|s| s.hand_count),
        Amount::LifeTotal(PlayerRel::You) => view
            .seat(payer)
            .map(|s| u32::try_from(s.life.max(0)).unwrap_or(0)),
        Amount::HighestOpponentHandSize if view.seats.len() == 2 => view
            .seats
            .iter()
            .filter(|s| s.loss.is_none() && s.player != payer)
            .map(|s| s.hand_count)
            .max()
            .or(Some(0)),
        Amount::DistinctManaValuesInZone {
            zone: ZoneSel::GraveyardYou,
            filter: &Filter::Any,
        } => {
            let gy = view.graveyards.get(usize::from(payer.get()))?;
            let mvs: std::collections::BTreeSet<u32> = gy.iter().map(|o| o.mana_value).collect();
            u32::try_from(mvs.len()).ok()
        }
        _ => None,
    }
}

/// `targeting::matches` evaluates `This` and `Another` against a spell in hand,
/// whereas `Condition::ControlCount` binds each candidate permanent as `source`.
fn uses_self_identity(filter: &Filter) -> bool {
    match filter {
        Filter::This | Filter::Another => true,
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(uses_self_identity),
        Filter::Not(inner) | Filter::AttachedTo(inner) => uses_self_identity(inner),
        _ => false,
    }
}

/// Whether the non-mana parts of an alternative cost could be paid right now.
///
/// `Engine::can_afford` is the function this mirrors, and it is a mirror
/// rather than a copy because it is held to a stricter rule: the engine may
/// answer from the `GameState`, and anything this cannot answer from the
/// **view** is refused. Every arm is named for the reason the engine's own
/// version names them — a twelfth [`CostPart`] has to be looked at here too,
/// rather than quietly falling into a wildcard and being declared payable.
fn parts_payable(view: &PlayerView, card: ObjectId, parts: &[CostPart]) -> bool {
    if !pitch_cards_payable(view, card, parts) || !permanents_payable(view, parts) {
        return false;
    }
    parts.iter().all(|part| match part {
        CostPart::PayLife(n) => view
            .seat(resource_player(view))
            .is_some_and(|seat| seat.life >= i32::from(*n)),
        // Collective availability, including distinct cards and permanents
        // for repeated parts, was established by the two matchers above; self-damage
        // is always payable regardless of life total (CR 118.3).
        CostPart::DealDamageToYou(_)
        | CostPart::ExileFromHand(Filter::HasColor(_))
        | CostPart::Sacrifice(_)
        | CostPart::ReturnToHand(_)
        | CostPart::TapN { .. } => true,
        // A pitch of another filter shape has no reader here, and the
        // remaining parts have no alternative cost in this pool. Keep
        // refusing them until a real card gives their view reader a test.
        CostPart::ExileFromHand(_)
        | CostPart::TapSelf
        | CostPart::UntapSelf
        | CostPart::ExertSelf
        | CostPart::SacrificeSelf
        | CostPart::ExilePermanent(_)
        | CostPart::Discard(_)
        | CostPart::DiscardRandom
        | CostPart::DiscardHand
        | CostPart::DiscardLastDrawnThisTurn
        | CostPart::PayEnergy(_)
        | CostPart::PayEnergyX
        | CostPart::TapOther(_)
        | CostPart::Crew(_)
        | CostPart::Saddle(_)
        | CostPart::ExileFromGraveyard(_)
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand
        | CostPart::RemoveCounterSelf { .. }
        | CostPart::RemoveCounterSelfX { .. }
        | CostPart::RemoveAnyCounter { .. }
        | CostPart::RemoveAnyCounterX { .. }
        | CostPart::PutCounterSelf { .. }
        | CostPart::PayLifeX
        | CostPart::PayCommanderColors
        | CostPart::TapX(_)
        | CostPart::SacrificeX(_)
        | CostPart::SacrificeAll(_)
        | CostPart::PutCounterOnOther { .. }
        | CostPart::ExileFromSingleGraveyard { .. }
        | CostPart::ExileTopOfLibrary(_)
        | CostPart::PayAttachedManaCost
        | CostPart::ReturnUnblockedAttacker
        | CostPart::SacrificeGrantor
        | CostPart::DiscardX(_)
        | CostPart::ExileFromGraveyardX(_)
        | CostPart::Forage
        | CostPart::CraftMaterials { .. } => false,
        CostPart::Behold(filter) => behold_payable(view, card, filter),
        CostPart::CollectEvidence(mv) => {
            let payer = resource_player(view);
            view.graveyards
                .get(usize::from(payer.get()))
                .is_some_and(|gy| {
                    gy.iter()
                        .filter(|o| o.id != card)
                        .map(|o| o.mana_value)
                        .fold(0u32, u32::saturating_add)
                        >= u32::from(*mv)
                })
        }
        CostPart::Either(a, b) => {
            (a.mana == baylee_core::mana::ManaCost::ZERO && parts_payable(view, card, a.parts))
                || (b.mana == baylee_core::mana::ManaCost::ZERO
                    && parts_payable(view, card, b.parts))
        }
    })
}

fn behold_payable(view: &PlayerView, card: ObjectId, filter: &Filter) -> bool {
    let payer = resource_player(view);
    let on_bf = view.battlefield_of(payer).any(|o| {
        o.id != card
            && !o.status.is_phased_out()
            && crate::targeting::matches(view, o, filter) == Some(true)
    });
    on_bf
        || baylee_client_core::decision::hand(view)
            .iter()
            .any(|h| h.id != card && hand_matches_behold(h, filter))
}

fn hand_matches_behold(hand: &baylee_view::HandObject, filter: &Filter) -> bool {
    match filter {
        Filter::Any | Filter::Another => true,
        Filter::And(parts) => parts.iter().all(|part| hand_matches_behold(hand, part)),
        Filter::Or(parts) => parts.iter().any(|part| hand_matches_behold(hand, part)),
        Filter::Not(inner) => !hand_matches_behold(hand, inner),
        Filter::HasType(types) => hand.types.intersects(*types),
        Filter::LacksType(types) => !hand.types.intersects(*types),
        Filter::HasColor(colors) => hand.colors.intersects(*colors),
        Filter::IsColorless => hand.colors.is_colorless(),
        Filter::Monocolored => hand.colors.len() == 1,
        Filter::IsCommander => hand.commander,
        Filter::HasSubtype(sub) => baylee_cards::by_index(hand.card.index).is_some_and(|def| {
            let face_idx = usize::from(hand.card.face);
            def.faces.get(face_idx).is_some_and(|face| {
                face.subtypes.contains(sub)
                    || (def
                        .keywords
                        .union(face.keywords)
                        .contains(baylee_cards_dsl::KeywordSet::CHANGELING)
                        && baylee_core::generated::subtypes::ALL_CREATURE_TYPES.contains(*sub))
            })
        }),
        Filter::HasSupertype(sup) => baylee_cards::by_index(hand.card.index).is_some_and(|def| {
            def.faces
                .get(usize::from(hand.card.face))
                .is_some_and(|face| face.supertypes.contains(*sup))
        }),
        _ => false,
    }
}

/// Match sacrifices, returns and nonsymbol taps to distinct permanents this seat
/// controls. Unknown projected filters refuse the alternative cost.
fn permanents_payable(view: &PlayerView, parts: &[CostPart]) -> bool {
    let mut requirements = Vec::new();
    for part in parts {
        match part {
            CostPart::Sacrifice(filter) | CostPart::ReturnToHand(filter) => {
                requirements.push((*filter, false));
            }
            CostPart::TapN { filter, count } => {
                for _ in 0..*count {
                    requirements.push((*filter, true));
                }
            }
            _ => {}
        }
    }
    // The payer's permanents, the controlled player's while this seat casts
    // for them (CR 724.5a).
    let payer = resource_player(view);
    let candidates = view
        .battlefield
        .iter()
        .filter(|object| object.controller == payer)
        .collect::<Vec<_>>();
    let mut assigned = vec![None; candidates.len()];
    (0..requirements.len()).all(|part| {
        let mut seen = vec![false; candidates.len()];
        assign_permanent(
            part,
            &requirements,
            &candidates,
            &mut assigned,
            &mut seen,
            view,
        )
    })
}

/// Find one permanent for a cost part, reassigning an earlier part through
/// an augmenting path when differently filtered parts compete for it.
fn assign_permanent(
    part: usize,
    requirements: &[(&Filter, bool)],
    candidates: &[&baylee_view::PublicObject],
    assigned: &mut [Option<usize>],
    seen: &mut [bool],
    view: &PlayerView,
) -> bool {
    let (filter, untapped) = requirements[part];
    for (index, object) in candidates.iter().enumerate() {
        if seen[index]
            || (untapped && object.status.is_tapped())
            || crate::targeting::matches(view, object, filter) != Some(true)
        {
            continue;
        }
        seen[index] = true;
        if assigned[index].is_none_or(|previous| {
            assign_permanent(previous, requirements, candidates, assigned, seen, view)
        }) {
            assigned[index] = Some(part);
            return true;
        }
    }
    false
}

/// Match every pitch part to a different object in the decision actor's hand.
/// Augmenting paths avoid both reusing one card and a greedy assignment that
/// hides a valid pairing when different parts ask for different colours.
fn pitch_cards_payable(view: &PlayerView, card: ObjectId, parts: &[CostPart]) -> bool {
    let filters = parts
        .iter()
        .filter_map(|part| match part {
            CostPart::ExileFromHand(Filter::HasColor(colors)) => Some(colors),
            _ => None,
        })
        .collect::<Vec<_>>();
    if filters.is_empty() {
        return true;
    }
    let candidates = baylee_client_core::decision::hand(view)
        .iter()
        .filter(|candidate| candidate.id != card)
        .map(|candidate| candidate.colors)
        .collect::<Vec<_>>();
    let mut matches = vec![None; candidates.len()];
    (0..filters.len()).all(|part| {
        let mut seen = vec![false; candidates.len()];
        assign_pitch(part, &filters, &candidates, &mut matches, &mut seen)
    })
}

/// Find a card for one part, moving an earlier assignment along an augmenting
/// path when necessary. `seen` bounds the search to each candidate once.
fn assign_pitch(
    part: usize,
    filters: &[&baylee_core::color::ColorSet],
    candidates: &[baylee_core::color::ColorSet],
    matches: &mut [Option<usize>],
    seen: &mut [bool],
) -> bool {
    for (index, colors) in candidates.iter().enumerate() {
        if seen[index] || !colors.intersects(*filters[part]) {
            continue;
        }
        seen[index] = true;
        if matches[index]
            .is_none_or(|previous| assign_pitch(previous, filters, candidates, matches, seen))
        {
            matches[index] = Some(part);
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::color::{Color, ColorSet};
    use baylee_core::generated::subtypes::land;
    use baylee_core::ids::{CardIndex, PlayerId};
    use baylee_core::types::{SubtypeSet, TypeSet};
    use baylee_view::{HandObject, PlayerView, PublicObject};

    /// The registry index of a card, by the name its front face prints.
    fn index_of(name: &str) -> CardIndex {
        baylee_cards::all()
            .find(|def| def.faces[0].name == name)
            .unwrap_or_else(|| panic!("{name} is not in the pool"))
            .index
    }

    /// A basic land, as the engine offers it: the CR 305.6 shortcut on a land
    /// with exactly one basic type.
    fn basic(slot: u32, name: &str, subtype: baylee_core::ids::SubtypeId) -> PublicObject {
        let mut o = token(slot, 0, name, 0, 0);
        o.card = Some(card_in_hand(slot, name, &[]).card);
        o.types = TypeSet::LAND;
        o.subtypes = SubtypeSet::from_slice(&[subtype]);
        o.power = None;
        o.toughness = None;
        o
    }

    /// One card in hand, named and coloured for real.
    fn card_in_hand(slot: u32, name: &str, colors: &[Color]) -> HandObject {
        HandObject {
            id: ObjectId::new(slot, 0),
            card: baylee_view::CardIdentity {
                index: index_of(name),
                print: baylee_core::ids::PrintRef::new(slot as u16),
                face: 0,
            },
            name: name.to_string(),
            mana_value: 0,
            colors: ColorSet::from_slice(colors),
            types: TypeSet::CREATURE,
            commander: false,
        }
    }

    /// `count` Plains on the table, a hand, and the engine offering every
    /// land's shortcut.
    fn table(hand: Vec<HandObject>, count: u32) -> (PlayerView, LegalActions) {
        lands_of(hand, count, "Plains", land::PLAINS)
    }

    /// The same with Islands, for the blue half of the pool.
    fn blue_table(hand: Vec<HandObject>, count: u32) -> (PlayerView, LegalActions) {
        lands_of(hand, count, "Island", land::ISLAND)
    }

    fn lands_of(
        hand: Vec<HandObject>,
        count: u32,
        name: &str,
        subtype: baylee_core::ids::SubtypeId,
    ) -> (PlayerView, LegalActions) {
        let lands: Vec<PublicObject> = (0..count).map(|i| basic(100 + i, name, subtype)).collect();
        let ids: Vec<ObjectId> = lands.iter().map(|l| l.id).collect();
        let mut view = ViewBuilder::new(2).with_battlefield(0, lands).build();
        view.hand = hand;
        let legal = LegalActions {
            mana_abilities: ids,
            ..LegalActions::default()
        };
        (view, legal)
    }

    fn kinds(modes: &[ReachableMode]) -> Vec<CastModeKind> {
        modes.iter().map(|m| m.kind).collect()
    }

    #[test]
    fn optional_flash_cost_replaces_the_sorcery_route_only_when_timing_requires_it() {
        let mut rout = card_in_hand(1, "Rout", &[Color::White]);
        rout.types = TypeSet::SORCERY;
        let (mut view, legal) = table(vec![rout.clone()], 7);
        assert_eq!(
            kinds(&reachable_modes(&view, &legal, rout.id)),
            [CastModeKind::Normal, CastModeKind::OptionalCost]
        );

        view.active = PlayerId::new(1);
        let modes = reachable_modes(&view, &legal, rout.id);
        assert_eq!(kinds(&modes), [CastModeKind::OptionalCost]);
        assert_eq!(modes[0].cost, ManaCost::parse("{5}{W}{W}"));
        assert_eq!(modes[0].plan.taps(), 7);

        view.sorcery_lock = Some(ObjectId::new(500, 0));
        assert!(reachable_modes(&view, &legal, rout.id).is_empty());
    }

    #[test]
    fn gloom_taxed_mana_remains_manual_but_is_not_planned_as_free() {
        let (mut view, mut legal) = table(vec![], 1);
        let source = legal.mana_abilities[0];
        view.seats[0].mana_pool.colorless = 3;
        legal.activation_increases.push((source, 3));
        assert!(crate::manasources::sources(&view, &legal).is_empty());
        assert!(!crate::manasources::offers(&view, &legal, source).is_empty());
    }

    #[test]
    fn gloom_plans_the_tax_for_normal_and_alternative_casts() {
        let hand = card_in_hand(1, "Savannah Lions", &[Color::White]);
        for (lands, expected) in [(3, 0), (4, 1)] {
            let (view, mut legal) = table(vec![hand.clone()], lands);
            legal
                .spell_increases
                .push((hand.id, CastModeKind::Normal, 3));
            let modes = reachable_modes(&view, &legal, hand.id);
            assert_eq!(modes.len(), expected);
            if let Some(mode) = modes.first() {
                assert_eq!(mode.cost, baylee_core::mana!("{3}{W}"));
                assert_eq!(mode.plan.taps(), 4);
            }
        }
        let solitude = card_in_hand(2, "Solitude", &[Color::White]);
        let (view, mut legal) = table(vec![hand, solitude.clone()], 3);
        legal
            .spell_increases
            .push((solitude.id, CastModeKind::Normal, 3));
        let modes = reachable_modes(&view, &legal, solitude.id);
        assert_eq!(kinds(&modes), vec![CastModeKind::Alternative(0)]);
        assert_eq!(modes[0].cost, baylee_core::mana!("{3}"));
        assert_eq!(modes[0].plan.taps(), 3);
    }

    #[test]
    fn tear_asunder_plans_the_complete_kicker_price_before_targeting() {
        let mut hand = card_in_hand(1, "Tear Asunder", &[Color::Green]);
        hand.types = TypeSet::INSTANT;
        let (mut view, mut legal) = lands_of(vec![hand.clone()], 2, "Forest", land::FOREST);
        let mut elf = token(300, 1, "Llanowar Elves", 1, 1);
        elf.card = Some(card_in_hand(300, "Llanowar Elves", &[Color::Green]).card);
        view.battlefield.push(elf);
        assert!(
            reachable_modes(&view, &legal, hand.id).is_empty(),
            "two mana cannot reach the only legal kicked targets"
        );
        for id in [102, 103] {
            let swamp = basic(id, "Swamp", land::SWAMP);
            legal.mana_abilities.push(swamp.id);
            view.battlefield.push(swamp);
        }
        let modes = reachable_modes(&view, &legal, hand.id);
        assert_eq!(kinds(&modes), [CastModeKind::Kicked]);
        assert_eq!(modes[0].plan.steps.len(), 4);
        assert_eq!(modes[0].cost, ManaCost::parse("{2}{B}{G}"));
        assert!(!crate::targeting::provably_targetless(&view, hand.card));
        let mut artifact = token(301, 1, "Artifact", 0, 0);
        artifact.types = TypeSet::ARTIFACT;
        artifact.card = Some(card_in_hand(301, "Mind Stone", &[]).card);
        view.battlefield.push(artifact);
        let modes = reachable_modes(&view, &legal, hand.id);
        assert_eq!(kinds(&modes), [CastModeKind::Normal, CastModeKind::Kicked]);
        assert_eq!(modes[0].plan.steps.len(), 2);
        assert_eq!(modes[1].plan.steps.len(), 4);
    }

    #[test]
    fn prototype_uses_three_swamps_and_disguise_keeps_the_land_option() {
        let (view, legal) = lands_of(
            vec![card_in_hand(1, "Phyrexian Fleshgorger", &[])],
            3,
            "Swamp",
            land::SWAMP,
        );
        let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(kinds(&modes), [CastModeKind::Prototype]);
        assert_eq!(modes[0].plan.steps.len(), 3);
        for (name, expected) in [
            (
                "Branch of Vitu-Ghazi",
                vec![CastModeKind::Disguise, CastModeKind::PlayLandFace(0)],
            ),
            (
                "Boggart Trawler",
                vec![CastModeKind::Normal, CastModeKind::PlayLandFace(1)],
            ),
        ] {
            let (view, mut legal) =
                lands_of(vec![card_in_hand(1, name, &[])], 3, "Swamp", land::SWAMP);
            legal.lands.push(ObjectId::new(1, 0));
            let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
            assert_eq!(kinds(&modes), expected, "{name}");
            assert!(
                modes[1].plan.steps.is_empty(),
                "playing a land floats no mana"
            );
        }
    }

    /// Overload is a way at its own price (CR 702.96a): Cyclonic Rift is
    /// offered as its targeted `{1}{U}` mode and its overloaded `{6}{U}`
    /// mode, each with the taps that pay for it, so the player picks before
    /// any land is tapped. A mode the mana or the board cannot carry is left
    /// off: two Islands reach only the cheap one, and with nothing of an
    /// opponent's to bounce only the overload stands.
    #[test]
    fn cyclonic_rift_offers_its_overload_beside_its_targeted_mode() {
        let rift = || vec![card_in_hand(1, "Cyclonic Rift", &[Color::Blue])];
        let with_target = |(mut view, legal): (PlayerView, LegalActions)| {
            let mut theirs = token(50, 1, "Grizzly Bears", 2, 2);
            theirs.types = TypeSet::CREATURE;
            view.battlefield.push(theirs);
            (view, legal)
        };

        let (view, legal) = with_target(blue_table(rift(), 7));
        let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(
            kinds(&modes),
            [CastModeKind::Mode(0), CastModeKind::Mode(1)],
            "both modes, and no mode-less printed cast"
        );
        assert_eq!(modes[0].plan.taps(), 2, "the targeted mode taps two");
        assert_eq!(modes[1].plan.taps(), 7, "the overload taps all seven");

        let (view, legal) = with_target(blue_table(rift(), 2));
        let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(kinds(&modes), [CastModeKind::Mode(0)], "two Islands");

        let (view, legal) = blue_table(rift(), 7);
        let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(
            kinds(&modes),
            [CastModeKind::Mode(1)],
            "nothing of theirs to target: only the overload"
        );
    }

    /// The measurement in the report: seven open mana and Reveillark's two
    /// ways both payable, which is the moment nothing asked.
    #[test]
    fn reveillark_with_six_open_mana_has_two_ways_to_be_cast() {
        let hand = vec![card_in_hand(1, "Reveillark", &[Color::White])];
        let (view, legal) = table(hand, 6);
        let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(
            kinds(&modes),
            vec![CastModeKind::Normal, CastModeKind::Alternative(0)],
            "printed {{4}}{{W}} and evoke {{5}}{{W}}, both reachable with six lands"
        );
        assert_eq!(modes[0].plan.steps.len(), 5, "the printed cost taps five");
        assert_eq!(modes[1].plan.steps.len(), 6, "evoke taps every one of them");
    }

    /// Five is enough for the printed cost and not for the evoke, so there is
    /// nothing to ask and the click keeps the behaviour it always had.
    #[test]
    fn reveillark_with_five_open_mana_has_only_the_printed_way() {
        let hand = vec![card_in_hand(1, "Reveillark", &[Color::White])];
        let (view, legal) = table(hand, 5);
        let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(kinds(&modes), vec![CastModeKind::Normal]);
    }

    /// The other direction of the same defect: the free way is reachable with
    /// no mana at all, so a seat with five lands has **two** ways and was
    /// being given the free one in silence.
    #[test]
    fn solitude_with_five_open_mana_offers_the_printed_cost_as_well() {
        let hand = vec![
            card_in_hand(1, "Solitude", &[Color::White]),
            card_in_hand(2, "Reveillark", &[Color::White]),
        ];
        let (view, legal) = table(hand, 5);
        let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(
            kinds(&modes),
            vec![CastModeKind::Normal, CastModeKind::Alternative(0)]
        );
        assert!(
            modes[1].plan.is_empty(),
            "a pitch costs no mana, so it plans no taps"
        );
    }

    /// The pitch is a cost like any other, and a hand with nothing white in
    /// it cannot pay it. Offering the row anyway would be a chooser that
    /// lies: the engine would not offer the way, and the answer this client
    /// had remembered would fit nothing in its list.
    #[test]
    fn solitude_alone_in_a_hand_cannot_pitch_to_itself() {
        let hand = vec![card_in_hand(1, "Solitude", &[Color::White])];
        let (view, legal) = table(hand, 5);
        let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(kinds(&modes), vec![CastModeKind::Normal]);

        // …and a hand whose other card is the wrong colour is the same
        // answer, which is what says the filter is being read rather than
        // the hand merely counted.
        let hand = vec![
            card_in_hand(1, "Solitude", &[Color::White]),
            card_in_hand(2, "Force of Will", &[Color::Blue]),
        ];
        let (view, legal) = table(hand, 5);
        let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(kinds(&modes), vec![CastModeKind::Normal]);
    }

    /// Force of Negation's pitch is only legal on somebody else's turn
    /// (`AltCondition::NotYourTurn`), and the view is what says whose turn it
    /// is.
    #[test]
    fn a_condition_on_an_alternative_cost_is_read_off_the_view() {
        let hand = vec![
            card_in_hand(1, "Force of Negation", &[Color::Blue]),
            card_in_hand(2, "Force of Will", &[Color::Blue]),
        ];
        let (mut view, legal) = blue_table(hand, 3);

        view.active = PlayerId::new(0);
        let mine = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(kinds(&mine), vec![CastModeKind::Normal], "my own turn");

        view.active = PlayerId::new(1);
        let theirs = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(
            kinds(&theirs),
            vec![CastModeKind::Normal, CastModeKind::Alternative(0)],
            "not my turn, so the pitch is on offer"
        );
    }

    /// The pool-wide claim this reader rests on, nailed down rather than
    /// written in a comment: every alternative cost in the registry is built
    /// out of the parts [`parts_payable`] reads. The day a card prints one
    /// that is not, this fails — which is the point, because the honest
    /// answer then is to decide what that card's row should say, not to let
    /// it quietly vanish from a chooser.
    #[test]
    fn every_alternative_cost_in_the_pool_is_one_this_reader_can_evaluate() {
        let mut unreadable = Vec::new();
        for def in baylee_cards::all() {
            for face in def.faces {
                for alt in face.alternative_costs {
                    let readable = alt.cost.parts.iter().all(|part| match part {
                        CostPart::PayLife(_)
                        | CostPart::Sacrifice(_)
                        | CostPart::ReturnToHand(_)
                        | CostPart::TapN { .. } => true,
                        CostPart::ExileFromHand(filter) => {
                            matches!(filter, Filter::HasColor(_))
                        }
                        _ => false,
                    });
                    if !readable {
                        unreadable.push(face.name);
                    }
                }
            }
        }
        assert!(
            unreadable.is_empty(),
            "these cards print an alternative cost this reader refuses: {unreadable:?}"
        );
    }

    #[test]
    fn sunscour_requires_two_distinct_white_pitch_cards() {
        let sunscour = card_in_hand(1, "Sunscour", &[Color::White]);
        let white = card_in_hand(2, "Savannah Lions", &[Color::White]);
        let another_white = card_in_hand(3, "Serra Avenger", &[Color::White]);
        let green = card_in_hand(4, "Llanowar Elves", &[Color::Green]);
        for (fodder, pitchable) in [
            (vec![white.clone()], false),
            (vec![white.clone(), green], false),
            (vec![white, another_white], true),
        ] {
            let mut hand = vec![sunscour.clone()];
            hand.extend(fodder);
            let (view, legal) = table(hand, 0);
            assert_eq!(
                reachable_modes(&view, &legal, sunscour.id)
                    .iter()
                    .any(|mode| mode.kind == CastModeKind::Alternative(0)),
                pitchable
            );
        }
    }

    #[test]
    fn fireblast_requires_two_distinct_mountains_for_its_alternative_cost() {
        let fireblast = card_in_hand(1, "Fireblast", &[Color::Red]);
        for (mountains, pitchable) in [(1, false), (2, true)] {
            let (view, legal) = lands_of(
                vec![fireblast.clone()],
                mountains,
                "Mountain",
                land::MOUNTAIN,
            );
            assert_eq!(
                reachable_modes(&view, &legal, fireblast.id)
                    .iter()
                    .any(|mode| mode.kind == CastModeKind::Alternative(0)),
                pitchable
            );
        }
    }

    /// Each Island a card returns is one of its own: Gush asks two and Daze one.
    #[test]
    fn daze_and_gush_return_distinct_islands_for_their_alternative_cost() {
        for (name, needed) in [("Daze", 1), ("Gush", 2)] {
            let spell = card_in_hand(1, name, &[Color::Blue]);
            for islands in 0..=needed {
                let (view, legal) = lands_of(vec![spell.clone()], islands, "Island", land::ISLAND);
                assert_eq!(
                    reachable_modes(&view, &legal, spell.id)
                        .iter()
                        .any(|mode| mode.kind == CastModeKind::Alternative(0)),
                    islands == needed,
                    "{name} with {islands}"
                );
            }
        }
    }

    #[test]
    fn sephara_requires_four_untapped_flying_creatures_and_one_white_mana() {
        let sephara = card_in_hand(1, "Sephara, Sky's Blade", &[Color::White]);
        for (fliers, pitchable) in [(3, false), (4, true)] {
            let (mut view, legal) = table(vec![sephara.clone()], 1);
            for slot in 0..fliers {
                let mut angel = token(200 + slot, 0, "Serra Angel", 4, 4);
                angel.card = Some(card_in_hand(200 + slot, "Serra Angel", &[Color::White]).card);
                angel.types = TypeSet::CREATURE;
                angel.keywords = baylee_cards_dsl::KeywordSet::FLYING.bits();
                view.battlefield.push(angel);
            }
            assert_eq!(
                reachable_modes(&view, &legal, sephara.id)
                    .iter()
                    .any(|mode| mode.kind == CastModeKind::Alternative(0)),
                pitchable
            );
        }
    }

    #[test]
    fn foretell_plans_its_fixed_action_cost_beside_the_normal_cast() {
        let hand = card_in_hand(1, "Doomskar", &[Color::White]);
        let id = hand.id;
        let (view, mut legal) = table(vec![hand], 5);
        legal.unpaid_foretell.push(id);
        let modes = reachable_modes(&view, &legal, id);
        assert_eq!(
            kinds(&modes),
            [CastModeKind::Normal, CastModeKind::Foretell]
        );
        assert_eq!(modes[1].cost, ManaCost::parse("{2}"));
        assert_eq!(modes[1].plan.taps(), 2);
        assert_eq!(modes[0].cost, ManaCost::parse("{3}{W}{W}"));
        legal.unpaid_foretell.clear();
        assert_eq!(
            kinds(&reachable_modes(&view, &legal, id)),
            [CastModeKind::Normal]
        );
    }

    #[test]
    fn snuff_out_requires_a_controlled_swamp_and_four_life_for_its_alternative_cost() {
        let mut snuff = card_in_hand(1, "Snuff Out", &[Color::Black]);
        snuff.types = TypeSet::INSTANT;
        let with_target = |(mut view, legal): (PlayerView, LegalActions)| {
            let mut theirs = token(50, 1, "Grizzly Bears", 2, 2);
            theirs.card = Some(card_in_hand(50, "Grizzly Bears", &[Color::Green]).card);
            theirs.colors = ColorSet::from_slice(&[Color::Green]);
            theirs.types = TypeSet::CREATURE;
            view.battlefield.push(theirs);
            (view, legal)
        };

        let (view, legal) = with_target(lands_of(vec![snuff.clone()], 4, "Swamp", land::SWAMP));
        let modes = reachable_modes(&view, &legal, snuff.id);
        assert_eq!(
            kinds(&modes),
            [CastModeKind::Normal, CastModeKind::Alternative(0)],
            "four Swamps and 20 life offer both the printed cost and the 4-life alternative"
        );
        assert_eq!(modes[0].plan.taps(), 4);
        assert!(modes[1].plan.is_empty());

        let (mut at_three_life, legal) =
            with_target(lands_of(vec![snuff.clone()], 4, "Swamp", land::SWAMP));
        at_three_life.seats[0].life = 3;
        assert_eq!(
            kinds(&reachable_modes(&at_three_life, &legal, snuff.id)),
            [CastModeKind::Normal],
            "three life cannot pay 4 life (CR 119.4)"
        );

        let (mut only_theirs, legal) = with_target(table(vec![snuff.clone()], 0));
        let mut their_swamp = basic(200, "Swamp", land::SWAMP);
        their_swamp.controller = PlayerId::new(1);
        only_theirs.battlefield.push(their_swamp);
        assert!(
            reachable_modes(&only_theirs, &legal, snuff.id).is_empty(),
            "an opponent's Swamp does not satisfy Condition::ControlCount"
        );

        let (mut phased_swamp, legal) =
            with_target(lands_of(vec![snuff.clone()], 1, "Swamp", land::SWAMP));
        phased_swamp.battlefield[0].status = baylee_view::ObjectStatus::PHASED_OUT;
        assert!(
            reachable_modes(&phased_swamp, &legal, snuff.id).is_empty(),
            "a phased-out Swamp is ignored on the battlefield"
        );
    }

    /// Rushwood Legate: "If an opponent controls an Island and you control a
    /// Forest, you may cast this spell without paying its mana cost" — both
    /// clauses, each on its own side of the table.
    #[test]
    fn a_legate_is_free_only_while_an_opponent_has_the_island_and_you_the_forest() {
        let legate = card_in_hand(1, "Rushwood Legate", &[Color::Green]);
        let their_island = || {
            let mut island = basic(200, "Island", land::ISLAND);
            island.controller = PlayerId::new(1);
            island.owner = PlayerId::new(1);
            island
        };

        let (mut both, legal) = lands_of(vec![legate.clone()], 1, "Forest", land::FOREST);
        both.battlefield.push(their_island());
        let modes = reachable_modes(&both, &legal, legate.id);
        assert_eq!(
            kinds(&modes),
            [CastModeKind::Alternative(0)],
            "one Forest cannot pay {{2}}{{G}}, and the free way is open"
        );
        assert!(modes[0].plan.is_empty(), "nothing is tapped for it");

        let (no_island, legal) = lands_of(vec![legate.clone()], 1, "Forest", land::FOREST);
        assert!(
            reachable_modes(&no_island, &legal, legate.id).is_empty(),
            "no opponent controls an Island"
        );

        let (mut my_island, legal) = lands_of(vec![legate.clone()], 1, "Forest", land::FOREST);
        my_island
            .battlefield
            .push(basic(200, "Island", land::ISLAND));
        assert!(
            reachable_modes(&my_island, &legal, legate.id).is_empty(),
            "my own Island is not an opponent's"
        );

        let (mut no_forest, legal) = lands_of(vec![legate.clone()], 1, "Plains", land::PLAINS);
        no_forest.battlefield.push(their_island());
        assert!(
            reachable_modes(&no_forest, &legal, legate.id).is_empty(),
            "no Forest of mine"
        );

        let (mut three_forests, legal) = lands_of(vec![legate.clone()], 3, "Forest", land::FOREST);
        three_forests.battlefield.push(their_island());
        assert_eq!(
            kinds(&reachable_modes(&three_forests, &legal, legate.id)),
            [CastModeKind::Normal, CastModeKind::Alternative(0)],
            "with three Forests the printed cost is a way as well"
        );
    }
}
