//! "Sacrifice this: it deals N damage." A free shot is held for what it can
//! kill or for the moment its creature dies anyway, never spent on a healthy
//! opponent's face, and never left unused when the creature goes.

use baylee_cards_dsl::{AbilityDef, Amount, CostPart, Effect, KeywordSet};
use baylee_core::ids::{Defender, ObjectId, PlayerId, TargetRef};
use baylee_core::mana::ManaCost;
use baylee_engine::choice::{LegalActions, PlayerAction};
use baylee_view::{PlayerView, PublicObject, Step};

use super::super::abilities_of_battlefield_object;
use super::read::{actor, has, hits_for_value, is_creature, life, opponents, toughness_left};
use crate::features4::EncodedCandidates4;

fn power(object: &PublicObject) -> i32 {
    i32::from(object.power.unwrap_or(0)).max(0)
}

/// The damage of a shot, for an ability that is one and costs no mana.
fn shot_of(ability: &AbilityDef) -> Option<i32> {
    let AbilityDef::Activated { cost, effects, .. } = ability else {
        return None;
    };
    if cost.mana != ManaCost::ZERO || !cost.parts.contains(&CostPart::SacrificeSelf) {
        return None;
    }
    match effects {
        [
            Effect::DealDamage {
                amount: Amount::Fixed(damage),
                ..
            },
        ] => i32::try_from(*damage).ok(),
        _ => None,
    }
}

/// The shot this activation is, if it is one.
pub(super) fn shot(view: &PlayerView, source: ObjectId, index: u32) -> Option<i32> {
    let object = view.object(source)?;
    shot_of(abilities_of_battlefield_object(object)?.get(index as usize)?)
}

/// Whether `object` dies without a shot: named by something of theirs on the
/// stack, or in a combat it loses.
fn doomed(view: &PlayerView, me: PlayerId, object: &PublicObject) -> bool {
    let named = view.stack.iter().any(|item| {
        item.controller != me
            && item.targets.iter().any(
                |target| matches!(target, TargetRef::Object(named) if named.object == object.id),
            )
    });
    let lost_combat = view.step == Step::DeclareBlockers
        && view.combat.blockers.iter().any(|block| {
            let (hit_by, fights) = if block.blocker == object.id {
                (view.object(block.attacker), true)
            } else {
                (view.object(block.blocker), block.attacker == object.id)
            };
            fights && hit_by.is_some_and(|other| power(other) >= toughness_left(object))
        });
    named || lost_combat
}

fn killable(view: &PlayerView, me: PlayerId, damage: i32) -> impl Iterator<Item = &PublicObject> {
    view.battlefield
        .iter()
        .filter(move |object| object.controller != me && is_creature(object))
        .filter(move |object| {
            toughness_left(object) <= damage && !has(object, KeywordSet::INDESTRUCTIBLE)
        })
}

/// Whether what comes at `me` this combat kills, unblocked as it stands.
fn dying_this_combat(view: &PlayerView, me: PlayerId) -> bool {
    view.step == Step::DeclareBlockers
        && view
            .combat
            .attackers
            .iter()
            .filter(|attacker| attacker.defending == Defender::Player(me))
            .filter(|attacker| {
                !view
                    .combat
                    .blockers
                    .iter()
                    .any(|block| block.attacker == attacker.creature)
            })
            .filter_map(|attacker| view.object(attacker.creature))
            .map(power)
            .sum::<i32>()
            >= life(view, me)
}

/// Whether a shot of `damage` from `source` has a use now.
fn shot_has_a_use(view: &PlayerView, me: PlayerId, source: &PublicObject, damage: i32) -> bool {
    doomed(view, me, source)
        || killable(view, me, damage).next().is_some()
        || opponents(view, me).any(|player| life(view, player) <= damage)
}

/// Whether this shot should wait: nothing for it to do yet.
pub(super) fn shot_is_wasted(view: &PlayerView, source: ObjectId, damage: i32) -> bool {
    let me = actor(view);
    view.object(source)
        .is_some_and(|object| !shot_has_a_use(view, me, object, damage))
}

/// The shot a pass should turn into: the creature dies anyway, or a creature
/// that connects for value is in range, or the seat is dying with attackers in
/// reach of what it can fire.
pub(super) fn shot_to_fire(
    cands: &EncodedCandidates4,
    view: &PlayerView,
    legal: &LegalActions,
) -> Option<usize> {
    let me = actor(view);
    let shots: Vec<(ObjectId, u32, i32)> = legal
        .abilities
        .iter()
        .filter_map(|&(source, index)| Some((source, index, shot(view, source, index)?)))
        .collect();
    let total: i32 = shots.iter().map(|shot| shot.2).sum();
    let dying = dying_this_combat(view, me)
        && view
            .combat
            .attackers
            .iter()
            .filter_map(|attacker| view.object(attacker.creature))
            .any(|object| toughness_left(object) <= total);
    let (source, index, _) = shots.iter().copied().find(|&(source, _, damage)| {
        view.object(source).is_some_and(|object| {
            dying || doomed(view, me, object) || killable(view, me, damage).any(hits_for_value)
        })
    })?;
    cands.candidates.iter().position(|cand| {
        matches!(cand.action, Some(PlayerAction::ActivateAbility { source: id, ability_index })
            if id == source && ability_index == index)
    })
}
