//! Creature Bond's death trigger, including last known information.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::{Cause, DamageTarget, GameEvent};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn setup() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = Duel::new(1100, forest())
        .battlefield(
            0,
            &[island(), island(), sol_ring(), index::NEVINYRRAL_S_DISK],
        )
        .battlefield(1, &[index::WALL_OF_AIR, llanowar_elves()])
        .hand(0, &[creature_bond(), index::TERROR])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let creature = on_battlefield(&e, P1, index::WALL_OF_AIR).unwrap();
    let rock = on_battlefield(&e, P0, sol_ring()).unwrap();
    let aura = attaches_only_to(&mut e, P0, creature_bond(), creature, rock);
    (e, creature, aura)
}

#[test]
fn creature_bond_remembers_toughness_from_an_anthem_destroyed_in_the_same_batch() {
    let mut e = Duel::new(1101, forest())
        .battlefield(
            0,
            &[island(), island(), sol_ring(), index::NEVINYRRAL_S_DISK],
        )
        .battlefield(1, &[index::GLORIOUS_ANTHEM, index::WALL_OF_AIR])
        .hand(0, &[creature_bond()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let creature = on_battlefield(&e, P1, index::WALL_OF_AIR).unwrap();
    let anthem = on_battlefield(&e, P1, index::GLORIOUS_ANTHEM).unwrap();
    let rock = on_battlefield(&e, P0, sol_ring()).unwrap();
    let aura = attaches_only_to(&mut e, P0, creature_bond(), creature, rock);
    assert_eq!(pt(&e, creature), (2, 6));
    pass_until(&mut e, |e| at_rest(e, P0));
    let disk = on_battlefield(&e, P0, index::NEVINYRRAL_S_DISK).unwrap();
    e.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    e.refresh_offer();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: disk,
            ability_index: 0,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    for object in [anthem, creature, aura] {
        assert_eq!(e.state().object(object).unwrap().zone, Zone::Graveyard);
    }
    assert_eq!(damage(&e, aura), vec![(P1, 6)]);
}

fn damage(e: &Engine<RegistryLookup>, aura: ObjectId) -> Vec<(PlayerId, u16)> {
    e.journal()
        .entries()
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::DamageDealt {
                source: Some(source),
                target: DamageTarget::Player(p),
                amount,
                is_combat: false,
            } if source == aura => Some((p, amount)),
            _ => None,
        })
        .collect()
}

fn move_to(e: &mut Engine<RegistryLookup>, object: ObjectId, to: ZoneLocation) {
    e.dev_state_mut(P0)
        .unwrap()
        .move_object(object, to, ZonePosition::Top, Cause::Effect)
        .unwrap();
}

fn collect(e: &mut Engine<RegistryLookup>) {
    let Pending::Priority { player, .. } = e.pending() else {
        panic!("priority")
    };
    let player = *player;
    e.apply(player, PlayerAction::PassPriority).unwrap();
}

#[test]
fn creature_bond_deals_toughness_to_creatures_controller_from_the_aura() {
    let (mut e, creature, aura) = setup();
    let life = e.state().players[1].life;
    kill(&mut e, creature);
    assert_eq!(damage(&e, aura), vec![(P1, 5)]);
    assert_eq!(e.state().players[1].life, life - 5);
    assert_eq!(e.state().object(aura).unwrap().zone, Zone::Graveyard);
}

#[test]
fn creature_bond_uses_counters_and_ignores_marked_damage() {
    let (mut e, creature, aura) = setup();
    let state = e.dev_state_mut(P0).unwrap();
    crate::replacement::put_counters(state, creature, CounterKind::P1P1, 2);
    state.object_mut(creature).unwrap().damage = 4;
    state.refresh_characteristics();
    kill(&mut e, creature);
    assert_eq!(damage(&e, aura), vec![(P1, 7)]);
}

#[test]
fn creature_bond_does_not_trigger_for_another_creature_or_exile() {
    let (mut e, creature, aura) = setup();
    let elf = on_battlefield(&e, P1, llanowar_elves()).unwrap();
    kill(&mut e, elf);
    assert!(damage(&e, aura).is_empty());
    move_to(&mut e, creature, ZoneLocation::Exile(P1));
    collect(&mut e);
    pass_until(&mut e, stack_is_empty);
    assert!(damage(&e, aura).is_empty());
}

#[test]
fn creature_bond_does_not_trigger_after_aura_is_destroyed() {
    let (mut e, creature, aura) = setup();
    kill(&mut e, aura);
    kill(&mut e, creature);
    assert!(damage(&e, aura).is_empty());
}

#[test]
fn creature_bond_triggers_when_aura_and_creature_die_together() {
    let (mut e, creature, aura) = setup();
    // One resolution destroys both before the next SBA/trigger scan.
    move_to(&mut e, creature, ZoneLocation::Graveyard(P1));
    move_to(&mut e, aura, ZoneLocation::Graveyard(P0));
    collect(&mut e);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(damage(&e, aura), vec![(P1, 5)]);
}

#[test]
fn creature_bond_keeps_death_values_when_creature_returns_before_resolution() {
    let (mut e, creature, aura) = setup();
    let state = e.dev_state_mut(P0).unwrap();
    crate::replacement::put_counters(state, creature, CounterKind::P1P1, 2);
    state.refresh_characteristics();
    move_to(&mut e, creature, ZoneLocation::Graveyard(P1));
    collect(&mut e);
    assert!(!e.state().zones.list(ZoneLocation::Stack).is_empty());
    move_to(&mut e, creature, ZoneLocation::Battlefield);
    e.dev_state_mut(P0).unwrap().refresh_characteristics();
    assert_eq!(
        e.state()
            .object(creature)
            .unwrap()
            .characteristics()
            .toughness,
        Some(5)
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(damage(&e, aura), vec![(P1, 7)]);
}

#[test]
fn creature_bond_keeps_death_values_when_creature_returns_in_the_same_resolution() {
    let (mut e, creature, aura) = setup();
    let state = e.dev_state_mut(P0).unwrap();
    crate::replacement::put_counters(state, creature, CounterKind::P1P1, 2);
    state.refresh_characteristics();
    move_to(&mut e, creature, ZoneLocation::Graveyard(P1));
    move_to(&mut e, creature, ZoneLocation::Battlefield);
    collect(&mut e);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(damage(&e, aura), vec![(P1, 7)]);
}

#[test]
fn creature_bond_uses_last_controller_not_owner_after_a_stolen_creature_returns() {
    let (mut e, creature, aura) = setup();
    let state = e.dev_state_mut(P0).unwrap();
    let filter = crate::effects::EffectFilter::object(state, creature);
    let timestamp = state.next_timestamp();
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Control,
        timestamp,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter,
        modifier: baylee_cards_dsl::Modifier::GainControl,
    });
    state.refresh_characteristics();
    assert_eq!(state.object(creature).unwrap().controller, P0);
    move_to(&mut e, creature, ZoneLocation::Graveyard(P1));
    collect(&mut e);
    move_to(&mut e, creature, ZoneLocation::Battlefield);
    e.dev_state_mut(P0).unwrap().refresh_characteristics();
    assert_eq!(e.state().object(creature).unwrap().controller, P1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(damage(&e, aura), vec![(P0, 5)]);
}

#[test]
fn creature_bond_handles_zero_and_negative_toughness_without_damage_or_life_gain() {
    for counters in [5, 7] {
        let (mut e, creature, aura) = setup();
        let life = e.state().players[1].life;
        let state = e.dev_state_mut(P0).unwrap();
        crate::replacement::put_counters(state, creature, CounterKind::M1M1, counters);
        state.refresh_characteristics();
        collect(&mut e);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(creature).unwrap().zone, Zone::Graveyard);
        assert_eq!(e.state().players[1].life, life);
        assert!(damage(&e, aura).is_empty());
    }
}

#[test]
fn creature_bond_handles_a_token_that_ceases_to_exist() {
    let (mut e, creature, aura) = setup();
    // A token copy of the Wall has the same characteristics, but no card.
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(creature)
        .unwrap()
        .card = None;
    kill(&mut e, creature);
    assert!(e.state().object(creature).is_none());
    assert_eq!(damage(&e, aura), vec![(P1, 5)]);
}

#[test]
fn creature_bond_event_context_changes_the_stack_hash() {
    let (mut e, creature, _) = setup();
    move_to(&mut e, creature, ZoneLocation::Graveyard(P1));
    collect(&mut e);
    let top = *e.state().zones.list(ZoneLocation::Stack).last().unwrap();
    let before = (e.state().snapshot_hash(), e.state().loop_signature());
    let object = e.dev_state_mut(P0).unwrap().object_mut(top).unwrap();
    let rider = object
        .riders
        .iter_mut()
        .find(|r| matches!(r, crate::object::Rider::EventDeparture(..)))
        .unwrap();
    *rider = crate::object::Rider::EventDeparture(P0, 6);
    assert_ne!(before.0, e.state().snapshot_hash());
    assert_ne!(before.1, e.state().loop_signature());
    pass_until(&mut e, stack_is_empty);
}

#[test]
fn creature_bond_real_removal_deals_untargeted_preventable_damage() {
    let (mut e, creature, aura) = setup();
    pass_until(&mut e, |e| at_rest(e, P0));
    let life = e.state().players[1].life;
    let state = e.dev_state_mut(P0).unwrap();
    let modifier = baylee_cards_dsl::Modifier::PlayerHexproof;
    let timestamp = state.next_timestamp();
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P1,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        filter: crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::Any),
        modifier,
    });
    state.shields.push(crate::prevention::Shield {
        protects: crate::prevention::Shielded::Player(P1),
        kind: crate::prevention::ShieldKind::Next(2),
        controller: P1,
    });
    state.players[0].mana_pool.add(ManaColor::Black, 2);
    e.refresh_offer();
    cast_with_floating(&mut e, P0, index::TERROR);
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![creature],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(damage(&e, aura), vec![(P1, 3)]);
    assert_eq!(e.state().players[1].life, life - 3);
    assert_eq!(e.state().object(creature).unwrap().zone, Zone::Graveyard);
}

#[test]
fn creature_bond_disk_destroys_aura_and_creature_in_either_internal_order() {
    for aura_first in [false, true] {
        let (mut e, creature, aura) = setup();
        pass_until(&mut e, |e| at_rest(e, P0));
        if aura_first {
            // The same position with the Aura created before its eventual host.
            let state = e.dev_state_mut(P0).unwrap();
            state.zones.remove(aura, ZoneLocation::Battlefield);
            state
                .zones
                .insert(aura, ZoneLocation::Battlefield, ZonePosition::Bottom, false);
            assert_eq!(
                state.zones.list(ZoneLocation::Battlefield).first(),
                Some(&aura)
            );
        }
        let disk = on_battlefield(&e, P0, index::NEVINYRRAL_S_DISK).unwrap();
        e.dev_state_mut(P0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, 1);
        e.refresh_offer();
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: disk,
                ability_index: 0,
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(creature).unwrap().zone, Zone::Graveyard);
        assert_eq!(e.state().object(aura).unwrap().zone, Zone::Graveyard);
        assert_eq!(damage(&e, aura), vec![(P1, 5)]);
    }
}
