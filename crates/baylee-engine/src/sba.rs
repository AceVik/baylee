//! State-based actions (CR 704), run as a fixpoint before every priority
//! grant. Each pass returns the actions it took; the engine repeats until
//! no action fires, then offers priority.

use crate::event::{Cause, GameEvent, LossReason};
use crate::object::{CounterKind, ObjectKind};
use crate::state::GameState;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_core::ids::PlayerId;
use baylee_core::types::{SupertypeSet, TypeSet};

/// What the SBA pass needs from the engine flow.
#[derive(Debug, Default)]
pub struct SbaOutcome {
    /// Whether anything changed (re-run required).
    pub changed: bool,
    /// A pending legend-rule choice interrupts the fixpoint.
    pub legend_choice: Option<(PlayerId, Vec<baylee_core::ids::ObjectId>)>,
}

/// Runs one SBA pass over the state (CR 704.3 list, S2 subset):
/// player losses, lethal damage, loyalty, legend rule, counter
/// annihilation, token cleanup.
pub fn run(state: &mut GameState) -> SbaOutcome {
    let mut outcome = SbaOutcome::default();

    // --- Player losses (CR 704.5a-c) -----------------------------------
    for player in 0..state.players.len() {
        let p = PlayerId::new(player as u8);
        let (life, poison, empty_draw, has_lost) = {
            let pl = &state.players[player];
            (pl.life, pl.poison, pl.tried_empty_draw, pl.has_lost)
        };
        if has_lost {
            continue;
        }
        let reason = if life <= 0 {
            Some(LossReason::Life)
        } else if poison >= 10 {
            Some(LossReason::Poison)
        } else if empty_draw {
            Some(LossReason::EmptyDraw)
        } else {
            None
        };
        if let Some(reason) = reason {
            eliminate_player(state, p, reason);
            outcome.changed = true;
        }
    }

    // --- Lethal damage / zero toughness (CR 704.5f-g) -------------------
    let battlefield = state.zones.list(ZoneLocation::Battlefield).clone();
    for id in battlefield {
        let Some(obj) = state.object(id) else {
            continue;
        };
        if !obj.characteristics().types.contains(TypeSet::CREATURE) {
            // Planeswalkers with 0 loyalty (CR 704.5i).
            if obj.characteristics().types.contains(TypeSet::PLANESWALKER)
                && obj.counters.get(CounterKind::Loyalty) == 0
                && obj.kind == ObjectKind::Permanent
            {
                destroy(state, id);
                outcome.changed = true;
            }
            continue;
        }
        let toughness = obj.characteristics().toughness.unwrap_or(0);
        let dead = if toughness <= 0 {
            true
        } else {
            obj.damage >= toughness as u16
        };
        if dead {
            destroy(state, id);
            outcome.changed = true;
        }
    }

    // --- +1/+1 vs -1/-1 annihilation (CR 704.5q) -------------------------
    for id in state.zones.list(ZoneLocation::Battlefield).clone() {
        let Some(obj) = state.object(id) else {
            continue;
        };
        let (plus, minus) = (
            obj.counters.get(CounterKind::P1P1),
            obj.counters.get(CounterKind::M1M1),
        );
        if plus > 0 && minus > 0 {
            let cancel = plus.min(minus);
            if let Some(obj) = state.object_mut(id) {
                obj.counters.set(CounterKind::P1P1, plus - cancel);
                obj.counters.set(CounterKind::M1M1, minus - cancel);
            }
            outcome.changed = true;
        }
    }

    // --- Legend rule (CR 704.5j) ----------------------------------------
    for seat in 0..state.players.len() {
        let player = PlayerId::new(seat as u8);
        let mut by_name: rustc_hash::FxHashMap<u32, Vec<baylee_core::ids::ObjectId>> =
            rustc_hash::FxHashMap::default();
        for &id in state.zones.list(ZoneLocation::Battlefield) {
            let Some(obj) = state.object(id) else {
                continue;
            };
            if obj.controller == player
                && obj
                    .characteristics()
                    .supertypes
                    .contains(SupertypeSet::LEGENDARY)
                && obj.kind == ObjectKind::Permanent
            {
                by_name
                    .entry(obj.characteristics().name.get())
                    .or_default()
                    .push(id);
            }
        }
        for (_, mut group) in by_name {
            if group.len() > 1 {
                group.sort(); // deterministic option order (oldest slot first)
                outcome.legend_choice = Some((player, group));
                return outcome; // interrupt: player choice resolves first
            }
        }
    }

    // --- Tokens outside the battlefield cease to exist (CR 704.5d) ------
    let mut vanished = Vec::new();
    for (id, obj) in state.arena.iter() {
        let is_token_like = obj.card.is_none()
            && !matches!(obj.kind, ObjectKind::Emblem | ObjectKind::AbilityOnStack);
        if is_token_like && obj.zone != crate::zone::Zone::Battlefield {
            vanished.push(id);
        }
    }
    for id in vanished {
        let _ = state.arena.remove(id);
        outcome.changed = true;
    }

    outcome
}

/// Applies a legend-rule choice (the kept object survives, the rest go to
/// the graveyard).
pub fn apply_legend_choice(
    state: &mut GameState,
    player: PlayerId,
    keep: baylee_core::ids::ObjectId,
    options: &[baylee_core::ids::ObjectId],
) {
    debug_assert!(options.contains(&keep));
    let _ = player;
    for &id in options {
        if id != keep {
            destroy(state, id);
        }
    }
}

/// Moves a permanent to its owner's graveyard (destruction).
pub fn destroy(state: &mut GameState, id: baylee_core::ids::ObjectId) {
    let owner = state.object(id).map_or(PlayerId::new(0), |o| o.owner);
    if let Some(obj) = state.object_mut(id) {
        obj.kind = ObjectKind::Card;
        obj.damage = 0;
    }
    let _ = state.move_object(
        id,
        ZoneLocation::Graveyard(owner),
        ZonePosition::Top,
        Cause::StateBased,
    );
}

/// Eliminates a player (S2: mark + journal; CR 800.4 object cleanup is
/// refined with multiplayer polish — here their objects leave the game).
pub fn eliminate_player(state: &mut GameState, player: PlayerId, reason: LossReason) {
    state.players[player.get() as usize].has_lost = true;
    state
        .journal
        .record(GameEvent::PlayerLost { player, reason });
    // CR 800.4a (simplified): everything they own leaves the game.
    let owned: Vec<_> = state
        .arena
        .iter()
        .filter(|(_, o)| o.owner == player)
        .map(|(id, _)| id)
        .collect();
    for id in owned {
        let (zone, owner) = match state.object(id) {
            Some(o) => (o.zone, o.zone_owner.unwrap_or(o.owner)),
            None => continue,
        };
        let loc = ZoneLocation::of(zone, owner);
        state.zones.remove(id, loc);
        let _ = state.arena.remove(id);
    }
    let attackers: Vec<_> = state
        .combat
        .attackers
        .iter()
        .filter(|a| state.object(a.creature).is_some())
        .copied()
        .collect();
    let blockers: Vec<_> = state
        .combat
        .blockers
        .iter()
        .filter(|b| state.object(b.blocker).is_some() && state.object(b.attacker).is_some())
        .copied()
        .collect();
    state.combat.attackers = attackers;
    state.combat.blockers = blockers;
}
