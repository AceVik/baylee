//! A pump in hand when the blocks are down. The tactical layer holds instants
//! through combat, and a fight a trick would turn is the moment they exist for.

use baylee_cards_dsl::{AbilityDef, Amount, Effect, KeywordSet};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::types::TypeSet;
use baylee_engine::choice::LegalActions;
use baylee_view::{PlayerView, PublicObject, Step};

use super::super::guard::cast_or_tap;
use super::super::lookup_card_identity;
use super::super::reading::reachable_spells;
use super::read::{actor, has, toughness_left};
use crate::features4::EncodedCandidates4;

fn power(object: &PublicObject) -> i32 {
    i32::from(object.power.unwrap_or(0)).max(0)
}

/// What a plain pump of these effects adds, as `(power, toughness)`.
fn pump_of(effects: &[Effect]) -> Option<(i32, i32)> {
    let [
        Effect::PumpTarget {
            power: Amount::Fixed(power),
            toughness: Amount::Fixed(toughness),
            ..
        },
    ] = effects
    else {
        return None;
    };
    Some((i32::try_from(*power).ok()?, i32::try_from(*toughness).ok()?))
}

/// The pump the card `id` is, read from its spell.
pub(super) fn pump_of_card(view: &PlayerView, id: ObjectId) -> Option<(i32, i32)> {
    let identity = lookup_card_identity(view, id)?;
    let card = baylee_cards::by_index(identity.index)?;
    card.abilities_for_face(usize::from(identity.face))
        .iter()
        .find_map(|ability| match ability {
            AbilityDef::Spell { effects, .. } => pump_of(effects),
            _ => None,
        })
}

/// Whether `pump` turns the fight of `mine` with `theirs`: it lives where it would
/// have died, or kills where it would not have.
fn turns_the_fight(mine: &PublicObject, theirs: &PublicObject, pump: (i32, i32)) -> bool {
    let hit = power(theirs);
    let tough = toughness_left(mine);
    let deadly = has(theirs, KeywordSet::DEATHTOUCH) && hit > 0;
    let saved = hit >= tough && hit < tough + pump.1 && !deadly;
    let target = toughness_left(theirs);
    let finishes = !has(theirs, KeywordSet::INDESTRUCTIBLE)
        && power(mine) < target
        && power(mine) + pump.0 >= target;
    saved || finishes
}

/// The seat's creatures whose one-on-one fight this combat `pump` would turn.
pub(super) fn turned_by(view: &PlayerView, me: PlayerId, pump: (i32, i32)) -> Vec<ObjectId> {
    if view.step != Step::DeclareBlockers {
        return Vec::new();
    }
    let lone = |attacker: ObjectId| {
        view.combat
            .blockers
            .iter()
            .filter(|block| block.attacker == attacker)
            .count()
            == 1
    };
    view.combat
        .blockers
        .iter()
        .filter(|block| lone(block.attacker))
        .filter_map(|block| {
            let (attacker, blocker) = (view.object(block.attacker)?, view.object(block.blocker)?);
            let (mine, theirs) = if super::super::is_ally(me, attacker.controller) {
                (attacker, blocker)
            } else {
                (blocker, attacker)
            };
            (super::super::is_ally(me, mine.controller) && turns_the_fight(mine, theirs, pump))
                .then_some(mine.id)
        })
        .collect()
}

/// What a pass in the blocks is turned to: the trick that turns a fight, or the
/// tap towards it.
pub(super) fn trick_to_cast(
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    legal: &LegalActions,
) -> Option<usize> {
    if !view.stack.is_empty() || view.owed.is_some() {
        return None;
    }
    let me = actor(view);
    let instant = |id: ObjectId| {
        view.hand
            .iter()
            .any(|card| card.id == id && card.types.contains(TypeSet::INSTANT))
    };
    let tricks: Vec<ObjectId> = reachable_spells(view, legal)
        .into_iter()
        .filter(|id| instant(*id))
        .filter(|id| {
            pump_of_card(view, *id).is_some_and(|pump| !turned_by(view, me, pump).is_empty())
        })
        .collect();
    if tricks.is_empty() {
        return None;
    }
    cast_or_tap(cands, logits, view, legal, &tricks)
}
