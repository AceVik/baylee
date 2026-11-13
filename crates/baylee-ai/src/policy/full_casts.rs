//! The house paying as every other planner pays
//! ([`crate::AIProfile::FULL_CASTS`], generation 10).
//!
//! Before it the house kept a reading of its own (`spend_on`, `pay_owed`,
//! `plan_for_activation` beside this module), frozen in `sharp-2026-11`,
//! while the client, the seat and the trained AI's menu moved to one
//! planner (`crate::spend`). What the house missed by it (observed faults
//! 70, 72 and 73):
//!
//! - restricted mana (CR 106.6) was read off the view, one tap a plan and
//!   none of it once floating; the engine now names, for each cast,
//!   activation and payment window, which restricted taps and which
//!   restricted mana floating may pay it (`LegalActions::restricted_taps`,
//!   `restricted_floating`), and a conversion is planned beside it;
//! - a Phyrexian symbol was black (or blue, …) only: Dismember beside lands
//!   that make no black was never cast at twenty life (CR 107.4f);
//! - Druids' Repository was one mana however many counters it held;
//! - a second Rishadan Port paid the first one's `{1}`;
//! - a card it may cast from exile (an Adventure's creature, CR 716.3d; a
//!   foretold card, CR 702.143a; a permission's card) or a disturb face in
//!   its graveyard (CR 702.146a) was cast only where its mana already
//!   floated, never tapped for. The engine names each way such a card may be
//!   cast, at its whole price (`LegalActions::cast_ways`); every way is
//!   planned here, scored as the face that way puts on the stack.
//!
//! What is cast, and when, is the same reading as before: the same scores,
//! the same reserve, the same X, kicker and buyback aims. Only how the mana
//! is found, and which cards are walked, differ.

use baylee_cards_dsl::{CostPart, FaceDef};
use baylee_client_core::decision::resource_player;
use baylee_client_core::manaplan::{self, Source, Tap};
use baylee_core::ids::ObjectId;
use baylee_core::mana::{ManaCost, ManaPayment};
use baylee_core::types::TypeSet;
use baylee_engine::choice::{CastModeKind, LegalActions, PlayerAction, Spend};
use baylee_view::{CardIdentity, PlayerView};

use super::{
    aim, face, has_instant_speed, identity, offers, own_casts, own_main_phase, plan_survives,
    spell_cost, spell_price, split_restricted, stillborn, usable,
};
use crate::HeuristicAgent;
use crate::spend::{self, Mana};

/// The life a Phyrexian symbol paid with life leaves the seat at the least:
/// the floor the answer to "pay 2 life?" keeps (`life > 2 + 5`).
pub(crate) const PHYREXIAN_FLOOR: i32 = 6;

/// One way of casting a card the planner may tap toward.
#[derive(Clone, Copy, Debug)]
struct Way {
    id: ObjectId,
    kind: CastModeKind,
    /// The face this way puts on the stack (CR 601.3e): a disturb face, an
    /// Adventure's creature, or the card as it is.
    card: CardIdentity,
    /// The engine's whole price for a card in a graveyard or in exile
    /// (`LegalActions::cast_ways`); `None` for a card priced as before, off
    /// the card and the view.
    price: Option<ManaCost>,
}

/// Whether `card` lies in a zone whose casting routes and whole prices the
/// engine names: this seat's graveyard, any exile zone, or a visible library
/// top. In particular, a top route's zero remaining mana must not be replaced
/// by the card's printed mana cost.
fn in_a_pile(view: &PlayerView, card: ObjectId) -> bool {
    let caster = resource_player(view);
    view.graveyards
        .get(usize::from(caster.get()))
        .is_some_and(|zone| zone.iter().any(|o| o.id == card))
        || view.exile.iter().flatten().any(|o| o.id == card)
        || view.library_tops.iter().any(|o| o.id == card)
}

/// Every way this seat could cast a card now or once its mana is made: its
/// hand, commanders and priced graveyard cards as before, and each card the
/// offer names from a graveyard, an exile zone or a visible library top
/// (`castable`, `payable`), one way for each the engine names, cheapest first.
fn ways(view: &PlayerView, legal: &LegalActions) -> Vec<Way> {
    let mut seen: Vec<ObjectId> = Vec::new();
    let mut out: Vec<Way> = Vec::new();
    let offered = legal.castable.iter().chain(&legal.payable).copied();
    for id in own_casts(view).chain(offered) {
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        let Some(front) = identity(view, id) else {
            continue;
        };
        let named: Vec<(CastModeKind, ManaCost)> = if in_a_pile(view, id) {
            let mut named: Vec<(CastModeKind, ManaCost)> = legal
                .cast_ways
                .iter()
                .filter(|(card, _)| *card == id)
                .map(|(_, way)| (way.kind, way.cost))
                .collect();
            named.sort_by_key(|(_, cost)| cost.cmc());
            named
        } else {
            Vec::new()
        };
        if named.is_empty() {
            // A card in a pile the engine names no way for is priced off
            // the view, as before: a graveyard card by its flashback.
            out.push(Way {
                id,
                kind: CastModeKind::Normal,
                card: front,
                price: None,
            });
            continue;
        }
        for (kind, cost) in named {
            let card = match kind {
                CastModeKind::Face(at) | CastModeKind::LibraryTopLife { face: at, .. } => {
                    CardIdentity {
                        face: u8::try_from(at).unwrap_or(front.face),
                        ..front
                    }
                }
                _ => front,
            };
            out.push(Way {
                id,
                kind,
                card,
                price: Some(cost),
            });
        }
    }
    out
}

/// The fixed life this exact library-top route will pay: the announced face's
/// mana value, mandatory additional life, and the life in a kicker already
/// carried by the route. The remaining mana is planned separately.
fn library_top_life(kind: CastModeKind, face: &FaceDef) -> Option<i32> {
    let CastModeKind::LibraryTopLife { life, kicked, .. } = kind else {
        return None;
    };
    let fixed = |parts: &[CostPart]| {
        parts.iter().fold(0_i32, |total, part| match part {
            CostPart::PayLife(amount) => total.saturating_add(i32::from(*amount)),
            _ => total,
        })
    };
    let mandatory = fixed(face.mandatory_additional_costs);
    let optional = kicked.then(|| {
        face.additional_costs
            .iter()
            .fold(0_i32, |total, cost| total.saturating_add(fixed(cost.parts)))
    });
    Some(
        i32::try_from(life)
            .unwrap_or(i32::MAX)
            .saturating_add(mandatory)
            .saturating_add(optional.unwrap_or(0)),
    )
}

/// The tap that makes the first mana of `step`.
const fn tap_of(step: &manaplan::Step) -> PlayerAction {
    match step.tap {
        Tap::Intrinsic => PlayerAction::ActivateManaAbility {
            source: step.source,
        },
        Tap::Ability(ability_index) => PlayerAction::ActivateAbility {
            source: step.source,
            ability_index,
        },
    }
}

/// The sources the shared planner reads, from one reading of the offer: the
/// unrestricted ones as the house reads them, with a counter-paid mana
/// ability once for each activation its counters pay, and the restricted
/// taps, of which the engine says what each may pay for.
fn mana_sources(view: &PlayerView, legal: &LegalActions) -> (Vec<Source>, Vec<Source>) {
    let (restricted, plain) = split_restricted(offers(view, legal));
    let mut plain = usable(view, legal, plain);
    spend::repeat_counter_sources(view, &mut plain);
    (
        plain,
        restricted.into_iter().map(|offer| offer.source).collect(),
    )
}

impl HeuristicAgent {
    /// [`HeuristicAgent::spend_on`] from generation 10: the same choice of
    /// spell, paid through [`crate::spend`] and walked over every way the
    /// engine names (module docs).
    #[allow(clippy::too_many_lines)]
    pub(super) fn spend_on_full(
        &self,
        view: &PlayerView,
        legal: &LegalActions,
        reserve: u32,
        score: impl Fn(&PlayerView, ObjectId, CardIdentity, u64) -> Option<i64>,
    ) -> Option<PlayerAction> {
        let seat = view.seat(resource_player(view))?;
        let pool = &seat.mana_pool;
        let (plain, restricted) = mana_sources(view, legal);
        let keep = spend::keepers(view, legal, None);
        let life = Some(seat.life.saturating_sub(PHYREXIAN_FLOOR));
        let main = own_main_phase(view);
        let available = pool.total() + plain.iter().map(|s| u64::from(s.amount)).sum::<u64>();
        let mana = Mana {
            pool,
            plain: &plain,
            restricted: &restricted,
        };
        let floating = Mana {
            pool,
            plain: &[],
            restricted: &[],
        };
        let mut best: Option<(i64, PlayerAction)> = None;
        for way in ways(view, legal) {
            let Way { id, kind, card, .. } = way;
            let Some(f) = face(card) else {
                continue;
            };
            if f.types.contains(TypeSet::LAND) {
                continue;
            }
            let library_life = library_top_life(kind, f);
            if library_life.is_some_and(|payment| {
                i64::from(seat.life) - i64::from(payment) < i64::from(PHYREXIAN_FLOOR)
            }) {
                continue;
            }
            let library_top = library_life.is_some();
            let instant = has_instant_speed(card, f);
            // A card the engine names from a pile has had its timing asked
            // already (`castable`, `payable`).
            if way.price.is_none()
                && !legal.castable.contains(&id)
                && (!main && !instant || !f.castable_from_hand)
            {
                continue;
            }
            if !legal.castable_once_paid(id) {
                continue;
            }
            if view.sorcery_lock.is_some() && !main {
                continue;
            }
            let Some(score) = score(view, id, card, available).filter(|score| *score > 0) else {
                continue;
            };
            let budget = if main && !instant {
                available.saturating_sub(u64::from(reserve))
            } else {
                available
            };
            let printed = way.price.unwrap_or_else(|| {
                spell_price(
                    view,
                    id,
                    Some(card),
                    legal,
                    CastModeKind::Normal,
                    spell_cost(view, id, f),
                )
            });
            // The engine's library route is already the whole remaining mana
            // price, including any announced kicker and modal costs. Its X is
            // fixed at zero, so neither aim nor exact-X targeting may raise it.
            let (cost, floats_first) = if library_top {
                (printed.with_x(0), false)
            } else {
                aim(
                    view,
                    legal,
                    card,
                    f,
                    printed,
                    &plain,
                    u32::try_from(budget).unwrap_or(u32::MAX),
                )
            };
            let zero_x = library_top && f.mana_cost.has_variable();
            // An instant or a sorcery whose X the mana makes no more than 0
            // waits for more (from generation 12): a Stream of Life, a
            // Volcanic Geyser or an Energy Bolt at X = 0 is a card spent on
            // nothing.
            if self.profile.plays(crate::AIProfile::EQUIPMENT_AND_X)
                && (zero_x || printed.has_variable() && cost.cmc() == printed.with_x(0).cmc())
                && f.types.intersects(TypeSet::INSTANT.union(TypeSet::SORCERY))
            {
                continue;
            }
            let cost = if library_top {
                cost
            } else {
                let Some(cost) = self.exact_x_cost(view, id, card, &printed, cost) else {
                    continue;
                };
                cost
            };
            let zero = f.mana_cost.with_x(0);
            let (rules_printed, rules_cost) = if library_top {
                (&f.mana_cost, &zero)
            } else {
                (&printed, &cost)
            };
            if stillborn(f, rules_printed, rules_cost)
                || self.unreachable_x_graveyard_target(view, id, card, rules_printed, rules_cost)
            {
                continue;
            }
            if !crate::granted::viable_cast(view, legal, &cost, &plain) {
                continue;
            }
            let paid = Spend::cast(id, kind);
            let planned = || spend::plan_keeping(paid, &cost, mana, legal, life, None, &keep);
            // Cast once the pool pays. A Phyrexian symbol makes a spell
            // castable off life alone (CR 107.4f), so with one it is cast
            // only where the planner, which reaches for mana before life,
            // has nothing left to tap. What the engine says is castable
            // beyond what the planner reads (a convoke, a delve) is cast as
            // before.
            let cast_now = legal.castable.contains(&id)
                && (!floats_first
                    || spend::plan_for(paid, &cost, floating, legal, None, None)
                        .is_some_and(|plan| plan.steps.is_empty()))
                && (cost.phyrexian_count() == 0
                    || planned().is_some_and(|plan| plan.steps.is_empty()));
            let spell_harm = self.spell_total_harm(view, Some(id), card, f);
            let (action, restricted_spent) = if cast_now {
                (PlayerAction::CastSpell { card: id }, 0)
            } else {
                let Some(plan) = planned() else {
                    continue;
                };
                if !plan_survives(view, &plan.steps, spell_harm) {
                    continue;
                }
                let Some(step) = plan.steps.first() else {
                    continue;
                };
                let named = legal.restricted_taps_for(paid);
                let restricted_spent: u32 = plan
                    .steps
                    .iter()
                    .filter(|step| {
                        matches!(step.tap, Tap::Ability(index) if named.contains(&(step.source, index)))
                    })
                    .filter_map(|step| restricted.iter().find(|s| s.id == step.source))
                    .map(|s| u32::from(s.amount))
                    .sum();
                (tap_of(step), restricted_spent)
            };
            // Restricted mana pays this spell and could hold up nothing
            // else, so it is not counted against the reserve.
            let spent = cost.cmc().saturating_sub(restricted_spent);
            if main && !instant && available.saturating_sub(u64::from(spent)) < u64::from(reserve) {
                continue;
            }
            if best.as_ref().is_none_or(|(value, _)| score > *value) {
                best = Some((score, action));
            }
        }
        best.map(|(_, action)| action)
    }
}

/// [`super::pay_owed`] from generation 10: the price owed in a payment window
/// (CR 605.3a), planned with the restricted mana the engine names for the
/// window (`Spend::Owed`) and every counter of a counter-paid mana ability.
pub(crate) fn pay_owed_full(view: &PlayerView, legal: &LegalActions) -> Option<PlayerAction> {
    let owed = match view.owed? {
        ManaPayment::Fixed(cost) => cost,
        ManaPayment::AnyAmount { preventable_damage } => {
            ManaCost::from_symbol_generic(preventable_damage)
        }
    };
    if view.awaiting != Some(view.seat) {
        return None;
    }
    let seat = view.seat(resource_player(view))?;
    let (plain, restricted) = mana_sources(view, legal);
    let mana = Mana {
        pool: &seat.mana_pool,
        plain: &plain,
        restricted: &restricted,
    };
    let normal = spend::plan_for(Spend::Owed, &owed, mana, legal, None, None);
    if normal.is_none() && matches!(view.owed, Some(ManaPayment::Fixed(_))) {
        return crate::granted::pay_fixed(view, legal, &owed, &plain);
    }
    let plan = normal.or_else(|| {
        // Partial prevention still helps when the full amount is unreachable.
        if matches!(view.owed, Some(ManaPayment::AnyAmount { .. })) {
            let one_more = seat.mana_pool.total() - seat.mana_pool.restricted_total() + 1;
            spend::plan_for(
                Spend::Owed,
                &ManaCost::from_symbol_generic(u32::try_from(one_more).ok()?),
                mana,
                legal,
                None,
                None,
            )
        } else {
            None
        }
    })?;
    if !plan_survives(view, &plan.steps, 0) {
        return None;
    }
    plan.steps.first().map(tap_of)
}

/// Whether to pay a Phyrexian symbol of this seat's own cast with 2 life
/// (CR 107.4f), from generation 10: never where the mana floating and the
/// seat's untapped sources pay the whole cost, which the planner reached
/// for before life; otherwise where the seat keeps more than
/// [`PHYREXIAN_FLOOR`] life after it. Before it every such question was
/// answered with life above seven, so a Dismember whose black the house had
/// tapped was paid with four life and the black emptied unspent.
pub(crate) fn pays_phyrexian_with_life(view: &PlayerView, cost: &ManaCost, amount: u32) -> bool {
    let Some(seat) = view.seat(resource_player(view)) else {
        return false;
    };
    let mana_pays = manaplan::plan(
        &cost.with_phyrexian_settled(0),
        &seat.mana_pool,
        &super::remaining_sources(view),
    )
    .is_some();
    !mana_pays && i64::from(seat.life) - i64::from(amount) >= i64::from(PHYREXIAN_FLOOR)
}
