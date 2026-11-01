//! Combat damage: who strikes in which step, the divisions players are asked for, and the assignment (CR 510).

use super::{OwedDivision, active_damage_recipient, bands_with_other_filters};
use crate::event::DamageTarget;
use crate::state::GameState;
use baylee_cards_dsl::KeywordSet as K;
use baylee_core::ids::ObjectId;

/// Whether a creature deals its combat damage in the given step
/// (CR 510.4): first strikers in the first step, everyone else in the
/// regular one, double strikers in both.
fn strikes_now(state: &GameState, creature: ObjectId, first_strike_step: bool) -> bool {
    let Some(obj) = state.object(creature) else {
        return false;
    };
    if let Some((first, eligible)) = &state.combat.prepared_damage {
        return *first == first_strike_step && eligible.contains(&(creature, obj.version));
    }
    let kw = obj.characteristics().keywords;
    if first_strike_step {
        kw.contains(K::FIRST_STRIKE) || kw.contains(K::DOUBLE_STRIKE)
    } else {
        kw.contains(K::DOUBLE_STRIKE)
            || state
                .combat
                .first_strikers
                .as_ref()
                .is_none_or(|first| !first.contains(&(creature, obj.version)))
    }
}

/// The same assignment eligibility for previews, divisions and final damage.
/// Keep the strike-step participants separate: assigning no damage does not
/// change which step a first striker belongs to (CR 510.4).
fn assigns_damage_now(state: &GameState, creature: ObjectId, first: bool) -> bool {
    strikes_now(state, creature, first) && !assigns_no_combat_damage(state, creature)
}

/// CR 510.4 fixes the strike-step participants before assignments. Revealing
/// a creature during assignment cannot create another damage step or remove
/// its already-required assignment. Returns whether any permanent turned up.
pub(crate) fn prepare_damage_step(state: &mut GameState, first: bool) -> bool {
    if state.combat.prepared_damage.is_some() {
        return false;
    }
    let mut creatures: Vec<_> = state
        .combat
        .attackers()
        .iter()
        .map(|a| a.creature)
        .chain(blocking_creatures(state))
        .collect();
    creatures.sort_unstable();
    creatures.dedup();
    let eligible: Vec<_> = creatures
        .iter()
        .copied()
        .filter(|&id| strikes_now(state, id, first))
        .filter_map(|id| state.object(id).map(|object| (id, object.version)))
        .collect();
    if first {
        state.combat.first_strikers = Some(eligible.clone());
    }
    state.combat.prepared_damage = Some((first, eligible));
    // Preview only establishes which sources would assign nonzero damage;
    // it neither spends existing divisions nor applies damage/prevention.
    let mut assignments = Vec::new();
    for attacker in state.combat.attackers() {
        if assigns_damage_now(state, attacker.creature, first) {
            assign_attacker_damage(state, attacker.creature, attacker.blocked, &mut assignments);
        }
    }
    for blocker in blocking_creatures(state) {
        if assigns_damage_now(state, blocker, first) {
            for (attacker, amount) in shares(
                state,
                blocker,
                &live_blocked(state, blocker),
                power_of(state, blocker),
            ) {
                assign(
                    &mut assignments,
                    blocker,
                    DamageTarget::Object(attacker),
                    amount,
                );
            }
        }
    }
    let mut changed = false;
    for assignment in assignments {
        if assignment.amount > 0 {
            changed |= state.reveal_masked(assignment.source);
        }
    }
    if changed {
        state.refresh_characteristics();
    }
    changed
}

/// How much damage from `source` is lethal to `target` right now
/// (CR 702.19b): toughness minus damage already marked, or 1 if the source
/// has deathtouch (CR 702.2b — *any* nonzero damage is lethal).
fn lethal_damage(state: &GameState, source: ObjectId, target: ObjectId) -> i16 {
    if has_keyword(state, source, K::DEATHTOUCH) {
        return 1;
    }
    let Some(obj) = state.object(target) else {
        return 1;
    };
    let toughness = obj.characteristics().toughness.unwrap_or(0).max(0);
    (toughness - obj.damage as i16).max(1)
}

pub(super) fn has_keyword(
    state: &GameState,
    id: ObjectId,
    kw: baylee_cards_dsl::KeywordSet,
) -> bool {
    state
        .object(id)
        .is_some_and(|o| o.characteristics().keywords.contains(kw))
}

fn power_of(state: &GameState, id: ObjectId) -> i16 {
    state
        .object(id)
        .and_then(|o| o.characteristics().power)
        .unwrap_or(0)
        .max(0)
}

/// Whether a creature has banding or "bands with other [quality]" paired with
/// a matching creature in `group` as damage is assigned (CR 702.22j–k).
fn has_banding_in_group(state: &GameState, id: ObjectId, group: &[ObjectId]) -> bool {
    if has_keyword(state, id, K::BANDING) {
        return true;
    }
    let filters = bands_with_other_filters(state, id);
    if filters.is_empty() {
        return false;
    }
    group.iter().any(|&other| {
        other != id
            && state.object(other).is_some_and(|obj| {
                filters
                    .iter()
                    .any(|f| crate::eval::matches(f, state, obj, obj.controller, id))
            })
    })
}

/// The creatures blocking `attacker` that are still there, in declaration
/// order.
fn live_blockers(state: &GameState, attacker: ObjectId) -> Vec<ObjectId> {
    state
        .combat
        .blockers_of(attacker)
        .into_iter()
        .filter(|b| state.object(*b).is_some())
        .collect()
}

/// The attackers `blocker` blocks that are still there, in the order the
/// blocks were made.
fn live_blocked(state: &GameState, blocker: ObjectId) -> Vec<ObjectId> {
    state
        .combat
        .blocked_by(blocker)
        .into_iter()
        .filter(|a| state.object(*a).is_some())
        .collect()
}

/// Every blocking creature once, in the order each first blocked.
///
/// A creature blocking a band blocks each of its members (CR 702.22h), so
/// the declaration holds one pair per member, and it still deals its combat
/// damage once (CR 510.1d): the damage step walks blockers, not pairs.
fn blocking_creatures(state: &GameState) -> Vec<ObjectId> {
    let mut out: Vec<ObjectId> = Vec::new();
    for info in &state.combat.blockers {
        if !out.contains(&info.blocker) {
            out.push(info.blocker);
        }
    }
    out
}

/// The divisions of combat damage players still owe before this strike
/// step's damage can be dealt, attackers first and then blockers, which is
/// the order CR 510.1 has them announced in.
///
/// - an attacker blocked by two or more creatures has its damage divided
///   among them by its controller (CR 510.1c), or by the defending player
///   if one of them has banding (CR 702.22j);
/// - a blocker blocking two or more creatures, which only a band makes it
///   do, has its damage divided by the active player if one of them has
///   banding (CR 702.22k), and by its own controller if none has any more
///   (CR 510.1d).
///
/// An attacker with trample blocked by creatures without banding is not
/// asked: the engine assigns lethal damage to each blocker in declaration
/// order and the rest to what it attacks, which is one of the assignments
/// CR 702.19b lets its controller make (the simplification this module's
/// header names).
#[must_use]
pub fn divisions_owed(state: &GameState, first_strike_step: bool) -> Vec<OwedDivision> {
    let mut owed = Vec::new();
    for info in state.combat.attackers() {
        if !info.blocked || !assigns_damage_now(state, info.creature, first_strike_step) {
            continue;
        }
        let amount = power_of(state, info.creature);
        let blockers = live_blockers(state, info.creature);
        if amount <= 0 || blockers.len() < 2 || state.combat.division(info.creature).is_some() {
            continue;
        }
        // Every blocker is the defending player's: only they declare blocks.
        let banding = blockers
            .iter()
            .find(|b| has_banding_in_group(state, **b, &blockers))
            .and_then(|b| state.object(*b))
            .map(|o| o.controller);
        let chooser = match banding {
            Some(defending) => Some(defending),
            None if has_keyword(state, info.creature, K::TRAMPLE) => None,
            None => state.object(info.creature).map(|o| o.controller),
        };
        if let Some(chooser) = chooser {
            owed.push(OwedDivision {
                source: info.creature,
                chooser,
                recipients: blockers,
                amount,
            });
        }
    }
    for blocker in blocking_creatures(state) {
        if !assigns_damage_now(state, blocker, first_strike_step) {
            continue;
        }
        let amount = power_of(state, blocker);
        let blocked = live_blocked(state, blocker);
        if amount <= 0 || blocked.len() < 2 || state.combat.division(blocker).is_some() {
            continue;
        }
        let chooser = if blocked
            .iter()
            .any(|a| has_banding_in_group(state, *a, &blocked))
        {
            state.turn.active
        } else {
            let Some(controller) = state.object(blocker).map(|o| o.controller) else {
                continue;
            };
            controller
        };
        owed.push(OwedDivision {
            source: blocker,
            chooser,
            recipients: blocked,
            amount,
        });
    }
    owed
}

/// How `source`'s `power` goes to `recipients`: all of it to the one there
/// is, or the division a player chose (CR 510.1c–d, 702.22j–k).
///
/// Two or more recipients and no division is a caller that dealt damage
/// without asking (`divisions_owed`), which the engine never does; the
/// whole of it then goes to the first, which is one of the divisions the
/// player could have chosen.
fn shares(
    state: &GameState,
    source: ObjectId,
    recipients: &[ObjectId],
    power: i16,
) -> Vec<(ObjectId, i16)> {
    match recipients {
        [] => Vec::new(),
        [only] => vec![(*only, power)],
        [first, ..] => state.combat.division(source).map_or_else(
            || vec![(*first, power)],
            |d| {
                d.shares
                    .iter()
                    .copied()
                    .filter(|(r, _)| recipients.contains(r))
                    .collect()
            },
        ),
    }
}

/// Deals combat damage for one strike step.
///
/// `first_strike_step`: only first/double strikers deal damage; the regular
/// step skips first-strikers (double strikers deal in both).
///
/// Attackers and blockers are two separate passes on purpose. Folding the
/// blockers' damage into the attacker loop tied a blocker's strike step to
/// its *attacker's* keywords — a first-striking attacker made its ordinary
/// blocker strike first too, which is precisely the interaction first
/// strike exists to decide — and skipped blockers the attacker had run out
/// of damage to assign to, though CR 510.1d has every blocking creature
/// deal its damage regardless.
///
/// The divisions players chose for this step (`divisions_owed`) are spent
/// here and emptied, so a double striker's second step asks again.
pub(crate) fn collect_combat_damage(
    state: &mut GameState,
    first_strike_step: bool,
) -> Vec<crate::damage::Assignment> {
    prepare_damage_step(state, first_strike_step);
    let mut assignments = Vec::new();
    for info in state.combat.attackers() {
        if assigns_damage_now(state, info.creature, first_strike_step) {
            assign_attacker_damage(state, info.creature, info.blocked, &mut assignments);
        }
    }
    for blocker in blocking_creatures(state) {
        if !assigns_damage_now(state, blocker, first_strike_step) {
            continue;
        }
        let power = power_of(state, blocker);
        for (attacker, amount) in shares(state, blocker, &live_blocked(state, blocker), power) {
            assign(
                &mut assignments,
                blocker,
                DamageTarget::Object(attacker),
                amount,
            );
        }
    }
    state.combat.divisions.clear();
    state.combat.prepared_damage = None;
    assignments
}

/// "This creature assigns no combat damage this turn" (Floral Spuzzem; the
/// card overrides CR 510.1a's "assigns combat damage equal to its power",
/// CR 101.1): it is left out of the step's assignments altogether, so there
/// is nothing to deal, prevent or redirect.
fn assigns_no_combat_damage(state: &GameState, creature: ObjectId) -> bool {
    state.object(creature).is_some_and(|obj| {
        state.effects.iter().any(|fx| {
            fx.modifier == baylee_cards_dsl::Modifier::AssignsNoCombatDamage
                && crate::effects::applies_to(state, fx, obj)
        })
    })
}

/// Assign every source before prevention can change a creature's power.
fn assign_attacker_damage(
    state: &GameState,
    attacker: ObjectId,
    blocked: bool,
    out: &mut Vec<crate::damage::Assignment>,
) {
    let power = power_of(state, attacker);
    if power <= 0 {
        return;
    }
    let trample = has_keyword(state, attacker, K::TRAMPLE);
    let live = live_blockers(state, attacker);
    if blocked && (!trample || live.iter().any(|b| has_banding_in_group(state, *b, &live))) {
        for (blocker, amount) in shares(state, attacker, &live, power) {
            assign(out, attacker, DamageTarget::Object(blocker), amount);
        }
    } else if blocked {
        let mut remaining = power;
        for blocker in &live {
            if remaining <= 0 {
                break;
            }
            let assigned = remaining.min(lethal_damage(state, attacker, *blocker));
            assign(out, attacker, DamageTarget::Object(*blocker), assigned);
            remaining -= assigned;
        }
        assign_defender(state, out, attacker, remaining);
    } else {
        assign_defender(state, out, attacker, power);
    }
}

fn assign_defender(
    state: &GameState,
    out: &mut Vec<crate::damage::Assignment>,
    source: ObjectId,
    amount: i16,
) {
    let Some(to) = active_damage_recipient(state, source) else {
        return;
    };
    assign(out, source, to, amount);
}

fn assign(
    out: &mut Vec<crate::damage::Assignment>,
    source: ObjectId,
    recipient: DamageTarget,
    amount: i16,
) {
    if amount > 0 {
        out.push(crate::damage::Assignment {
            source,
            source_version: None,
            recipient,
            amount: amount as u32,
            is_combat: true,
        });
    }
}

#[cfg(test)]
pub(super) fn deal_combat_damage(state: &mut GameState, first_strike_step: bool) {
    let assignments = collect_combat_damage(state, first_strike_step);
    assert!(
        crate::damage::DamageWork::new(state, assignments)
            .advance(state)
            .is_none(),
        "fixture needs damage choices"
    );
}

/// True if the damage `source` is dealing can't be prevented
/// (`CombatDamageCantBePrevented`, CR 615.12): combat damage from an object
/// an effect says so of. Every prevention effect then does nothing to it,
/// protection's (CR 702.16e) as much as a shield's.
pub(crate) fn unpreventable(state: &GameState, source: ObjectId, is_combat: bool) -> bool {
    is_combat
        && state.object(source).is_some_and(|obj| {
            state.effects.iter().any(|fx| {
                matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::CombatDamageCantBePrevented
                ) && crate::effects::applies_to(state, fx, obj)
            })
        })
}
