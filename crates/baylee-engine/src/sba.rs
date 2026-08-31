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
#[allow(clippy::too_many_lines)] // the CR 704.3 list is naturally one long pass
pub fn run(state: &mut GameState) -> SbaOutcome {
    let mut outcome = SbaOutcome::default();

    // --- Player losses (CR 704.5a-c) -----------------------------------
    // Everybody Lives: no losses while the effect is active.
    let cant_lose = state
        .effects
        .iter()
        .any(|fx| matches!(fx.modifier, baylee_cards_dsl::Modifier::PlayersCantLose));
    for player in 0..state.players.len() {
        if cant_lose {
            break;
        }
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

    // --- Lethal damage / zero toughness (CR 704.5f-h) -------------------
    let battlefield = state.zones.list(ZoneLocation::Battlefield).clone();
    for id in &battlefield {
        let id = *id;
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
        // CR 704.5f: zero or less toughness puts it in the graveyard. This
        // is not destruction, so indestructible does not save it.
        if toughness <= 0 {
            destroy(state, id);
            outcome.changed = true;
            continue;
        }
        // Indestructible permanents can't be destroyed (CR 702.12b), which
        // covers both lethal damage and deathtouch.
        if obj
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE)
        {
            continue;
        }
        // CR 704.5g lethal damage, CR 704.5h deathtouch — one point from a
        // deathtouch source is lethal however big the creature is.
        if obj.damage >= toughness as u16 || obj.deathtouched {
            destroy(state, id);
            outcome.changed = true;
        }
    }
    // The deathtouch window is "since the last time state-based actions
    // were checked" (CR 704.5h), so this pass — which has now judged every
    // marked creature — closes it.
    for id in &battlefield {
        if let Some(obj) = state.object_mut(*id) {
            obj.deathtouched = false;
        }
    }

    // --- Attachments (CR 704.5m-p) --------------------------------------
    outcome.changed |= run_attachment_sbas(state);

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
        // Sakashima-style suppression: the legend rule doesn't apply to
        // permanents this player controls.
        let legend_off = state.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::LegendRuleOff)
                && fx.controller == player
        });
        if legend_off {
            continue;
        }
        // Group by name in battlefield (zone) order — no HashMap: hash
        // iteration order is build/platform-dependent, and this loop
        // decides WHICH choice a player sees first when two legend pairs
        // coexist, so it is part of the determinism contract.
        let mut names: Vec<u32> = Vec::new();
        let mut groups: Vec<Vec<baylee_core::ids::ObjectId>> = Vec::new();
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
                let name = obj.characteristics().name.get();
                if let Some(i) = names.iter().position(|&n| n == name) {
                    groups[i].push(id);
                } else {
                    names.push(name);
                    groups.push(vec![id]);
                }
            }
        }
        for mut group in groups {
            if group.len() > 1 {
                group.sort(); // deterministic option order (oldest slot first)
                outcome.legend_choice = Some((player, group));
                return outcome; // interrupt: player choice resolves first
            }
        }
    }

    // --- Tokens outside the battlefield cease to exist (CR 704.5d) ------
    // "Card-less" is not the same as "token": an emblem, an ability on the
    // stack and a *copy of a spell* (CR 707.10 — a copy is not a token)
    // all have no card behind them, and none of them may be swept up here.
    // The spell case mattered: a copy of a token-backed spell was deleted
    // by this pass before it could ever resolve.
    let mut vanished = Vec::new();
    for (id, obj) in state.arena.iter() {
        let is_token_like = obj.card.is_none()
            && !matches!(
                obj.kind,
                ObjectKind::Emblem | ObjectKind::AbilityOnStack | ObjectKind::Spell
            );
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

/// Attachment state-based actions (CR 704.5m–p).
///
/// An Aura attached to something illegal — or to nothing — is put into its
/// owner's graveyard; an Equipment or Fortification in the same position
/// simply becomes unattached and stays on the battlefield. Without this,
/// an aura outlived the creature it enchanted and kept granting its
/// effect, and equipment kept pointing at a dead object whose slot a later
/// permanent could reuse.
fn run_attachment_sbas(state: &mut GameState) -> bool {
    use baylee_core::types::TypeSet as T;
    let mut changed = false;
    let mut falling_off = Vec::new();
    let mut unattaching = Vec::new();
    for &id in state.zones.list(ZoneLocation::Battlefield) {
        let Some(obj) = state.object(id) else { continue };
        if obj.kind != ObjectKind::Permanent {
            continue;
        }
        let types = obj.characteristics().types;
        let is_aura = obj
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::enchantment::AURA);
        let is_equipment = obj
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::artifact::EQUIPMENT);
        if !is_aura && !is_equipment {
            continue;
        }
        // The host has to be a permanent on the battlefield; anything else
        // (destroyed, exiled, bounced, or never set) is an illegal
        // attachment.
        let host_ok = obj.attached_to.is_some_and(|host| {
            state.object(host).is_some_and(|h| {
                h.zone == crate::zone::Zone::Battlefield
                    && h.kind == ObjectKind::Permanent
                    // CR 303.4f: an Aura can't enchant an Aura it is
                    // attached to being itself; self-attachment is never
                    // legal for either kind.
                    && host != id
            })
        });
        if host_ok {
            continue;
        }
        // An Equipment that is also a creature (living weapon, an animated
        // Equipment) is not attached to anything and that is fine.
        if is_aura && !types.contains(T::CREATURE) {
            falling_off.push(id);
        } else if obj.attached_to.is_some() {
            unattaching.push(id);
        }
    }
    for id in falling_off {
        destroy(state, id);
        changed = true;
    }
    for id in unattaching {
        if let Some(obj) = state.object_mut(id) {
            obj.attached_to = None;
        }
        changed = true;
    }
    changed
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
        obj.deathtouched = false;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CardLookup;
    use baylee_core::ids::{CardIndex, PrintRef};
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatController, SeatSpec,
    };

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn card_index(oracle_id: &str) -> CardIndex {
        baylee_cards::by_oracle_id(oracle_id)
            .expect("registry contains the card")
            .index
    }

    fn forest() -> CardIndex {
        card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
    }
    fn elesh_norn() -> CardIndex {
        card_index("5ade11c0-41dd-4b6a-9f5b-c5903a3a0d7f")
    }
    fn mox_opal() -> CardIndex {
        card_index("de2440de-e948-4811-903c-0bbe376ff64d")
    }

    fn entry(card: CardIndex) -> DeckEntry {
        DeckEntry {
            card,
            print: PrintRef::new(0),
        }
    }

    /// Seat 0 controls two legend pairs at once: 2× Elesh Norn and
    /// 2× Mox Opal (Elesh Norn enters the battlefield list first).
    fn two_legend_pairs_preset(seed: u64) -> GamePreset {
        let deck: Vec<DeckEntry> = (0..60).map(|_| entry(forest())).collect();
        let mk = |bf: Vec<CardIndex>| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            deck: deck.clone(),
            sideboard: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: bf.into_iter().map(entry).collect(),
            emblems: vec![],
            team: None,
        };
        GamePreset {
            format: FormatId::Freeform,
            seed,
            dev_mode: false,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: vec![
                mk(vec![elesh_norn(), elesh_norn(), mox_opal(), mox_opal()]),
                mk(vec![]),
            ],
        }
    }

    /// With two legend pairs coexisting, the FIRST choice a player sees
    /// must be deterministic — it used to depend on hash iteration order.
    #[test]
    fn legend_choice_is_deterministic_with_two_pairs() {
        let mut first: Option<Vec<baylee_core::ids::ObjectId>> = None;
        for seed in [7, 42, 1337] {
            let mut state = GameState::from_preset(&two_legend_pairs_preset(seed), &RegistryLookup)
                .expect("game starts");
            let outcome = run(&mut state);
            let (player, group) = outcome.legend_choice.expect("a legend choice is due");
            assert_eq!(player, PlayerId::new(0));
            assert_eq!(group.len(), 2, "one pair is offered, not all four");
            // The offered pair is the Elesh Norns (first in zone order):
            // both objects carry her card index.
            for id in &group {
                let obj = state.object(*id).expect("offered object exists");
                assert_eq!(
                    obj.card.map(|c| c.index),
                    Some(elesh_norn()),
                    "the first pair in battlefield order is offered first"
                );
            }
            if let Some(prev) = &first {
                assert_eq!(*prev, group, "same choice across seeds and runs");
            }
            first = Some(group);
        }
    }
}
