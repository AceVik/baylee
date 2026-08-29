//! Structured combat state machine.
//!
//! S2: single-blocker damage assignment, first/double strike steps,
//! trample, lifelink, flying/reach/menace restrictions. Multi-blocker
//! ordering and assignment choices arrive with the full choice taxonomy
//! (M2); deathtouch tracking lands with keywords (M1.S3).

use crate::event::{DamageTarget, GameEvent};
use crate::object::{GameObject, Status};
use crate::state::GameState;
use baylee_cards_dsl::KeywordSet as K;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::types::TypeSet;

/// One declared attacker.
#[derive(Clone, Copy, Debug)]
pub struct AttackerInfo {
    /// The attacking creature.
    pub creature: ObjectId,
    /// The player it attacks.
    pub defending: PlayerId,
}

/// One declared blocker.
#[derive(Clone, Copy, Debug)]
pub struct BlockerInfo {
    /// The blocking creature.
    pub blocker: ObjectId,
    /// The attacker it blocks.
    pub attacker: ObjectId,
}

/// The combat phase's mutable state.
#[derive(Clone, Debug, Default)]
pub struct CombatState {
    /// Declared attackers.
    pub attackers: Vec<AttackerInfo>,
    /// Declared blockers.
    pub blockers: Vec<BlockerInfo>,
}

impl CombatState {
    /// Whether combat is underway.
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.attackers.is_empty()
    }

    /// Blockers assigned to an attacker, in declaration order.
    #[must_use]
    pub fn blockers_of(&self, attacker: ObjectId) -> Vec<ObjectId> {
        self.blockers
            .iter()
            .filter(|b| b.attacker == attacker)
            .map(|b| b.blocker)
            .collect()
    }

    /// Whether a creature is currently blocked.
    #[must_use]
    pub fn is_blocked(&self, attacker: ObjectId) -> bool {
        self.blockers.iter().any(|b| b.attacker == attacker)
    }
}

/// Whether `creature` may attack at all (untapped, a creature, not
/// summoning-sick).
#[must_use]
pub fn can_attack(state: &GameState, player: PlayerId, creature: ObjectId) -> bool {
    let Some(obj) = state.object(creature) else {
        return false;
    };
    obj.zone == crate::zone::Zone::Battlefield
        && obj.controller == player
        && obj.characteristics().types.contains(TypeSet::CREATURE)
        && !obj.status.contains(Status::TAPPED)
        && !summoning_sick(state, obj)
}

/// Summoning sickness (CR 302.6): a creature must be controlled
/// continuously since the beginning of its controller's most recent turn
/// (haste excepted).
#[must_use]
pub fn summoning_sick(state: &GameState, obj: &GameObject) -> bool {
    if obj
        .characteristics()
        .keywords
        .contains(baylee_cards_dsl::KeywordSet::HASTE)
    {
        return false;
    }
    obj.timestamp >= state.turn_start_timestamp
}

/// Whether `blocker` may block `attacker` (keyword restrictions included).
#[must_use]
pub fn can_block(
    state: &GameState,
    defending: PlayerId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    let (Some(b), Some(a)) = (state.object(blocker), state.object(attacker)) else {
        return false;
    };
    if b.zone != crate::zone::Zone::Battlefield
        || b.controller != defending
        || !b.characteristics().types.contains(TypeSet::CREATURE)
        || b.status.contains(Status::TAPPED)
    {
        return false;
    }
    let kw =
        |o: &GameObject, k: baylee_cards_dsl::KeywordSet| o.characteristics().keywords.contains(k);
    // Flying can only be blocked by flying/reach (CR 702.9).
    if kw(a, K::FLYING) && !kw(b, K::FLYING) && !kw(b, K::REACH) {
        return false;
    }
    // Menace requires two or more blockers (checked at declaration: a
    // single blocker is never legal).
    if kw(a, K::MENACE) {
        let blockers = state.combat.blockers_of(attacker).len();
        if blockers == 0 {
            return false;
        }
    }
    // Unblockable.
    if kw(a, K::UNBLOCKABLE) {
        return false;
    }
    true
}

/// Deals combat damage for all attackers (S2 assignment rules).
///
/// `first_strike_step`: only first/double strikers deal damage; the regular
/// step skips first-strikers (double strikers deal in both).
pub fn deal_combat_damage(state: &mut GameState, first_strike_step: bool) {
    let attackers = state.combat.attackers.clone();
    for info in attackers {
        let Some(a) = state.object(info.creature) else {
            continue;
        };
        let kw = a.characteristics().keywords;
        let deals_now = if first_strike_step {
            kw.contains(K::FIRST_STRIKE) || kw.contains(K::DOUBLE_STRIKE)
        } else {
            !kw.contains(K::FIRST_STRIKE) || kw.contains(K::DOUBLE_STRIKE)
        };
        if !deals_now {
            continue;
        }
        let power = a.characteristics().power.unwrap_or(0).max(0);
        let trample = kw.contains(K::TRAMPLE);
        let lifelink = kw.contains(K::LIFELINK);
        let controller = a.controller;
        let blockers = state.combat.blockers_of(info.creature);
        if blockers.is_empty() {
            // Unblocked: damage the defending player.
            deal_damage_to_player(state, info.creature, info.defending, power, true);
            if lifelink {
                gain_life(state, controller, power);
            }
        } else {
            let mut remaining = power;
            for blocker in &blockers {
                if remaining <= 0 {
                    break;
                }
                let toughness = state
                    .object(*blocker)
                    .and_then(|b| b.characteristics().toughness)
                    .unwrap_or(0)
                    .max(0);
                let damage_already = state.object(*blocker).map_or(0, |b| b.damage);
                let lethal_needed = (toughness - damage_already as i16).max(1);
                let assigned = if trample {
                    remaining.min(lethal_needed)
                } else {
                    remaining
                };
                deal_damage_to_object(state, info.creature, *blocker, assigned, true);
                remaining -= assigned;
                if lifelink {
                    gain_life(state, controller, assigned);
                }
                // Blocker hits back.
                let blocker_power = state
                    .object(*blocker)
                    .and_then(|b| b.characteristics().power)
                    .unwrap_or(0)
                    .max(0);
                if blocker_power > 0 {
                    deal_damage_to_object(state, *blocker, info.creature, blocker_power, true);
                }
            }
            if trample && remaining > 0 {
                deal_damage_to_player(state, info.creature, info.defending, remaining, true);
                if lifelink {
                    gain_life(state, controller, remaining);
                }
            }
        }
    }
}

fn deal_damage_to_player(
    state: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    amount: i16,
    is_combat: bool,
) {
    if amount <= 0 {
        return;
    }
    if prevent_from(state, source) {
        return;
    }
    let p = &mut state.players[player.get() as usize];
    let old = p.life;
    p.life -= i32::from(amount);
    let new = p.life;
    state.journal.record(GameEvent::LifeChanged {
        player,
        old,
        new,
        cause: crate::event::Cause::Spell,
    });
    state.journal.record(GameEvent::DamageDealt {
        source: Some(source),
        target: DamageTarget::Player(player),
        amount: amount as u16,
        is_combat,
    });
}

fn deal_damage_to_object(
    state: &mut GameState,
    source: ObjectId,
    target: ObjectId,
    amount: i16,
    is_combat: bool,
) {
    if amount <= 0 {
        return;
    }
    if prevent_from(state, source) || prevent_to(state, target) {
        return;
    }
    if let Some(obj) = state.object_mut(target) {
        obj.damage = obj.damage.saturating_add(amount as u16);
    }
    state.journal.record(GameEvent::DamageDealt {
        source: Some(source),
        target: DamageTarget::Object(target),
        amount: amount as u16,
        is_combat,
    });
}

/// True if the source object may not deal damage (PreventDamageFromIt).
fn prevent_from(state: &GameState, source: ObjectId) -> bool {
    state.effects.iter().any(|fx| {
        matches!(fx.modifier, baylee_cards_dsl::Modifier::PreventDamageFromIt)
            && matches!(&fx.filter, crate::effects::EffectFilter::ObjectIs(id) if *id == source)
    })
}

/// True if the target object may not be dealt damage (PreventDamageToIt).
fn prevent_to(state: &GameState, target: ObjectId) -> bool {
    state.effects.iter().any(|fx| {
        matches!(fx.modifier, baylee_cards_dsl::Modifier::PreventDamageToIt)
            && matches!(&fx.filter, crate::effects::EffectFilter::ObjectIs(id) if *id == target)
    })
}

fn gain_life(state: &mut GameState, player: PlayerId, amount: i16) {
    if amount <= 0 {
        return;
    }
    let p = &mut state.players[player.get() as usize];
    let old = p.life;
    p.life += i32::from(amount);
    let new = p.life;
    state.journal.record(GameEvent::LifeChanged {
        player,
        old,
        new,
        cause: crate::event::Cause::Spell,
    });
}
