//! The opening hand and the land drop, by what the hand can do with them.

use std::collections::BTreeMap;

use baylee_cards_dsl::EnterModifier;
use baylee_core::ids::ObjectId;
use baylee_core::types::TypeSet;
use baylee_engine::choice::{LegalActions, Pending};
use baylee_view::{Phase, PlayerView};

use super::super::reading::reachable_spells;
use super::super::{
    cost_colors_reachable, is_spell_worth_casting_now, is_true_land_card, land_card_colors,
    land_is_bouncer_or_dup_legend, lookup_card_identity, lookup_spell_cost, non_sac_source_colors,
};
use super::read::{actor, is_tapped};
use crate::features4::{EncodedCandidates4, untapped_mana_production};

/// A logit that puts a card on the bottom for certain.
const BOTTOM: f32 = 100.0;

/// How many spells [`low_spends`] weighs against each other.
const MOST_SPELLS: usize = 10;

fn lands_in_hand(view: &PlayerView) -> Vec<ObjectId> {
    view.hand
        .iter()
        .filter(|card| is_true_land_card(view, card.id))
        .map(|card| card.id)
        .collect()
}

/// A hand with one land or none goes back, unless it is already small: a
/// one-drop does not make it a hand.
pub(super) fn redraw(view: &PlayerView, pending: &Pending) -> bool {
    matches!(pending, Pending::Mulligan { taken, can_take: true, .. } if *taken <= 1)
        && lands_in_hand(view).len() <= 1
}

/// Puts spare lands on the bottom: of the cards kept, about half should be
/// lands. The commonest land goes first, so the colours stay. Returns whether
/// any card was marked.
pub(super) fn bottom_spare_lands(
    cands: &EncodedCandidates4,
    logits: &mut [f32],
    view: &PlayerView,
    pending: &Pending,
) -> bool {
    let Pending::MulliganBottom { count, .. } = pending else {
        return false;
    };
    let count = usize::from(*count);
    let kept = view.hand.len().saturating_sub(count);
    let lands = lands_in_hand(view);
    let spare = lands.len().saturating_sub(kept.div_ceil(2)).min(count);
    if spare == 0 {
        return false;
    }
    let mut copies: BTreeMap<u32, usize> = BTreeMap::new();
    let index_of = |id: &ObjectId| {
        view.hand
            .iter()
            .find(|card| card.id == *id)
            .map_or(0, |card| card.card.index.get())
    };
    for land in &lands {
        *copies.entry(index_of(land)).or_default() += 1;
    }
    let mut order = lands.clone();
    order.sort_by_key(|land| std::cmp::Reverse(copies.get(&index_of(land)).copied()));
    for land in order.iter().take(spare) {
        for (cand, logit) in cands.candidates.iter().zip(logits.iter_mut()) {
            if cand.object_id == Some(*land) {
                *logit = BOTTOM;
            }
        }
    }
    true
}

fn enters_tapped(view: &PlayerView, land: ObjectId) -> bool {
    lookup_card_identity(view, land)
        .and_then(|identity| {
            let card = baylee_cards::by_index(identity.index)?;
            let face = card.faces.get(usize::from(identity.face))?;
            Some(face.enter_modifiers.contains(&EnterModifier::Tapped))
        })
        .unwrap_or(false)
}

/// The most mana a set of `costs` spends within `mana`.
pub(super) fn best_spend(costs: &[u32], mana: u32) -> u32 {
    let mut reachable = vec![false; mana as usize + 1];
    reachable[0] = true;
    for &cost in costs {
        for total in (cost as usize..=mana as usize).rev() {
            if reachable[total - cost as usize] {
                reachable[total] = true;
            }
        }
    }
    reachable
        .iter()
        .rposition(|hit| *hit)
        .map_or(0, |at| at as u32)
}

/// The spells of this main phase outside every set that spends the most of the
/// seat's mana: casting one strands what a dearer set would have used. Mana floats
/// a land at a time, so a cheap spell reaches the menu first unless it goes.
pub(super) fn low_spends(view: &PlayerView, legal: &LegalActions) -> Vec<ObjectId> {
    let me = actor(view);
    if view.active != me
        || !view.stack.is_empty()
        || view.owed.is_some()
        || !matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
    {
        return Vec::new();
    }
    let instant = |id: ObjectId| {
        view.hand
            .iter()
            .find(|card| card.id == id)
            .is_none_or(|card| card.types.contains(TypeSet::INSTANT))
    };
    let spells: Vec<(ObjectId, u32)> = reachable_spells(view, legal)
        .into_iter()
        .filter(|id| !instant(*id) && is_spell_worth_casting_now(view, *id))
        .filter_map(|id| {
            let (cost, tax) = lookup_spell_cost(view, id)?;
            (!cost.has_variable()).then(|| (id, cost.with_x(0).cmc().saturating_add(tax)))
        })
        .take(MOST_SPELLS)
        .collect();
    if spells.len() < 2 {
        return Vec::new();
    }
    let pool = view
        .seat(me)
        .map_or(0, |seat| u32::try_from(seat.mana_pool.total()).unwrap_or(0));
    let mana = pool.saturating_add(untapped_mana_production(view, me));
    let (mut best, mut members) = (0_u32, 0_u32);
    for set in 1_u32..(1 << spells.len()) {
        let total: u32 = spells
            .iter()
            .enumerate()
            .filter(|(index, _)| set & (1 << index) != 0)
            .map(|(_, spell)| spell.1)
            .sum();
        if total <= mana && total > best {
            best = total;
            members = set;
        } else if total == best {
            members |= set;
        }
    }
    spells
        .iter()
        .enumerate()
        .filter(|(index, _)| members & (1 << index) == 0)
        .map(|(_, spell)| spell.0)
        .collect()
}

/// The lands not worth the drop: those that spend less of the hand this turn
/// than another would, a land that sends a land back while another does not,
/// and, with nothing to cast either way, one that enters untapped while a tapped
/// land could use the turn.
pub(super) fn poor_drops(view: &PlayerView, legal: &LegalActions) -> Vec<ObjectId> {
    let me = actor(view);
    if view.active != me
        || !view.stack.is_empty()
        || view.owed.is_some()
        || !matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
        || legal.lands.len() < 2
    {
        return Vec::new();
    }
    let lands: Vec<ObjectId> = legal
        .lands
        .iter()
        .copied()
        .filter(|land| is_true_land_card(view, *land))
        .collect();
    if lands.len() < 2 {
        return Vec::new();
    }
    let pool = view
        .seat(me)
        .map_or(0, |seat| u32::try_from(seat.mana_pool.total()).unwrap_or(0));
    let base = pool.saturating_add(untapped_mana_production(view, me));
    let mut colours = [false; 6];
    for object in view.battlefield_of(me).filter(|object| !is_tapped(object)) {
        for (have, source) in colours
            .iter_mut()
            .zip(non_sac_source_colors(view, object.id))
        {
            *have |= source;
        }
    }
    let spells: Vec<_> = view
        .hand
        .iter()
        .filter(|card| !card.types.contains(TypeSet::LAND))
        .filter_map(|card| lookup_spell_cost(view, card.id))
        .collect();
    let spend = |land: ObjectId| -> u32 {
        let tapped = enters_tapped(view, land);
        let mut reach = colours;
        if !tapped {
            for (have, source) in reach.iter_mut().zip(land_card_colors(view, land)) {
                *have |= source;
            }
        }
        let costs: Vec<u32> = spells
            .iter()
            .filter(|(cost, _)| cost_colors_reachable(cost, &reach))
            .map(|(cost, tax)| cost.with_x(0).cmc().saturating_add(*tax))
            .collect();
        best_spend(&costs, base + u32::from(!tapped))
    };
    let scores: Vec<u32> = lands.iter().map(|land| spend(*land)).collect();
    let best = scores.iter().copied().max().unwrap_or(0);
    let mut poor: Vec<ObjectId> = lands
        .iter()
        .zip(&scores)
        .filter(|(_, score)| **score < best)
        .map(|(land, _)| *land)
        .collect();
    let left: Vec<ObjectId> = lands
        .iter()
        .copied()
        .filter(|land| !poor.contains(land))
        .collect();
    let bouncers: Vec<ObjectId> = left
        .iter()
        .copied()
        .filter(|land| land_is_bouncer_or_dup_legend(view, me, *land))
        .collect();
    if bouncers.len() < left.len() {
        poor.extend(&bouncers);
    }
    let left: Vec<ObjectId> = left
        .into_iter()
        .filter(|land| !bouncers.contains(land))
        .collect();
    if best == 0 && left.iter().any(|land| enters_tapped(view, *land)) {
        poor.extend(left.iter().filter(|land| !enters_tapped(view, **land)));
    }
    poor
}
