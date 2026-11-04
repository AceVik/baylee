//! The prices a card can be reached at beside its printed one.
//!
//! A cast the seat taps for is cast off the pool, and the mode question
//! counts the pool (`cast_options`), so tapping for the printed cost took
//! every dearer way off the table before the question was asked: Journey of
//! Discovery with fourteen lands up was tapped for its `{2}{G}`, and at the
//! mode question the entwined `{4}{G}{G}` was gone.
//!
//! So each price the lands can pay is an option of its own, tapped for
//! before the cast, and the question then asks what it always asks, with
//! that way on it. Where each comes from depends on how the engine asks it:
//!
//! - **On the mode question**: entwine, overload and a mode with its own
//!   price, a kicker with its own targets, an alternative cost with mana in
//!   it, dash, mutate, a prototype, a back face. The engine lists these,
//!   legal targets asked and whole prices named (`LegalActions::cast_ways`),
//!   so no way is invented here.
//! - **After the cast begins**: a kicker (`YesNoPrompt::Kicker`) and replicate
//!   (`NumberPrompt::Replicate`). Their legality is the plain cast's, so the
//!   card's own cost is read. A kicker could also be paid in the window the
//!   "yes" opens, but replicate is bounded by the pool and is asked only
//!   when the pool already pays one more.

use baylee_client_core::manaplan::Plan;
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaCost;
use baylee_engine::choice::{CastModeKind, LegalActions};
use baylee_view::{CardIdentity, PlayerView};

use super::{Reach, ReachFrom};

/// How a [`Reach`] is cast when it is not the printed cost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    /// A way the mode question lists.
    Mode(CastModeKind),
    /// With its kicker, which is asked after the cast begins.
    Kicker,
    /// With its replicate cost paid this many times.
    Replicated(u32),
}

/// How often a replicate price is tried at most; a board runs out far sooner.
const MOST_REPLICATES: u32 = 50;

/// The most replicate prices shown for a card: once, and as often as the
/// lands pay. The number question asks for less out of the same taps.
fn replications(greatest: u32) -> impl Iterator<Item = u32> {
    (1..=greatest).filter(move |&n| n == 1 || n == greatest)
}

/// Whether the engine's list leaves a way at this printed price. Cyclonic
/// Rift with nothing to bounce is castable only overloaded, so `{1}{U}` tapped
/// for would stand before a mode question it pays for nothing on. A card the
/// list says nothing about stands, and so does a face-down cast, which has
/// its own price.
pub(super) fn stands(legal: &LegalActions, plain: &Reach) -> bool {
    let mut ways = legal
        .cast_ways
        .iter()
        .filter(|(id, _)| *id == plain.object)
        .peekable();
    plain.face_down
        || ways.peek().is_none()
        || ways.any(|(_, way)| way.cost.with_x(0) == plain.plan.cost)
}

/// Every reach beside `plain` the lands can pay. `pay` plans a whole price
/// for a card, with nothing further added to it.
pub(super) fn ways(
    view: &PlayerView,
    legal: &LegalActions,
    plain: &[Reach],
    pay: &dyn Fn(ObjectId, CardIdentity, ManaCost) -> Option<Plan>,
) -> Vec<Reach> {
    let mut out: Vec<Reach> = Vec::new();
    for (id, desc) in &legal.cast_ways {
        // The printed cost, a face-down cast and flashback have reaches of
        // their own.
        if matches!(
            desc.kind,
            CastModeKind::Normal | CastModeKind::Disguise | CastModeKind::Flashback
        ) {
            continue;
        }
        let Some((card, from)) = locate(view, *id) else {
            continue;
        };
        if !baylee_ai::spell_can_be_cast_now(view, legal, *id, card) {
            continue;
        }
        let Some(plan) = pay(*id, card, desc.cost.with_x(0)) else {
            continue;
        };
        // One option a price: the mode question says which way, and a way
        // at the printed price is the printed reach's already.
        let same = |reach: &Reach| reach.object == *id && reach.plan.cost == plan.cost;
        if plain.iter().chain(&out).any(same) {
            continue;
        }
        out.push(Reach {
            object: *id,
            card,
            from,
            face_down: false,
            way: Some(Way::Mode(desc.kind)),
            plan,
        });
    }
    for reach in plain.iter().filter(|reach| !reach.face_down) {
        let Some(face) = baylee_cards::by_index(reach.card.index)
            .and_then(|def| def.faces.get(usize::from(reach.card.face)))
        else {
            continue;
        };
        let with = |way: Way, plan: Plan| Reach {
            way: Some(way),
            plan,
            ..reach.clone()
        };
        let mana_only = face
            .additional_costs
            .iter()
            .all(|cost| cost.parts.is_empty() && cost.mana != ManaCost::ZERO);
        if !face.additional_costs.is_empty() && face.kicked_targets.is_none() && mana_only {
            let kicked = face
                .additional_costs
                .iter()
                .fold(reach.plan.cost, |cost, extra| cost.combine(&extra.mana));
            out.extend(pay(reach.object, reach.card, kicked).map(|plan| with(Way::Kicker, plan)));
        }
        if let Some(each) = face.replicate {
            let price = |times: u32| reach.plan.cost.combine_n(&each, times);
            let greatest = (1..=MOST_REPLICATES)
                .take_while(|&times| pay(reach.object, reach.card, price(times)).is_some())
                .last()
                .unwrap_or(0);
            out.extend(replications(greatest).filter_map(|times| {
                let plan = pay(reach.object, reach.card, price(times))?;
                Some(with(Way::Replicated(times), plan))
            }));
        }
    }
    out
}

/// The card `id` is, and where it would be cast from.
fn locate(view: &PlayerView, id: ObjectId) -> Option<(CardIdentity, ReachFrom)> {
    if let Some(card) = baylee_client_core::decision::hand(view)
        .iter()
        .find(|card| card.id == id)
    {
        return Some((card.card, ReachFrom::Hand));
    }
    let in_zone = |zones: &[Vec<baylee_view::PublicObject>]| {
        zones
            .iter()
            .flatten()
            .find(|object| object.id == id)
            .and_then(|object| object.card)
    };
    in_zone(&view.command)
        .map(|card| (card, ReachFrom::CommandZone))
        .or_else(|| in_zone(&view.graveyards).map(|card| (card, ReachFrom::Graveyard)))
        .or_else(|| in_zone(&view.exile).map(|card| (card, ReachFrom::Exile)))
}
