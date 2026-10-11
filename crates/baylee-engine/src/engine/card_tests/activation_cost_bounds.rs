//! Exhaust and defined/minimum X exercised through real card activations.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn choose(e: &mut Engine<RegistryLookup>, object: ObjectId) {
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![object],
        },
    )
    .unwrap();
}
fn offered(e: &Engine<RegistryLookup>, id: ObjectId, ability: u32) -> bool {
    matches!(e.pending(), Pending::Priority { legal, .. } if legal.abilities.contains(&(id, ability)))
}
fn move_to(e: &mut Engine<RegistryLookup>, object: ObjectId, zone: ZoneLocation) {
    e.dev_state_mut(P0)
        .unwrap()
        .move_object(object, zone, ZonePosition::Top, Cause::Setup)
        .unwrap();
    e.refresh_offer();
}

#[test]
fn exhaust_liliana_stays_spent_across_turns_and_fizzles_but_resets_on_blink() {
    for fizzle in [false, true] {
        let mut e = prepared(
            &[index::GRIZZLY_BEARS, index::SERRA_ANGEL],
            &[index::LILIANA_THE_REPENTANT],
            &[],
        );
        let liliana = on_battlefield(&e, P0, index::LILIANA_THE_REPENTANT).unwrap();
        let bear = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
        let angel = in_hand(&e, P0, index::SERRA_ANGEL).unwrap();
        move_to(&mut e, bear, ZoneLocation::Graveyard(P0));
        move_to(&mut e, angel, ZoneLocation::Graveyard(P0));
        let library = library_size(&e, P0);
        activate(&mut e, P0, index::LILIANA_THE_REPENTANT, 1);
        assert!(e.state().object(liliana).unwrap().riders.is_empty());
        choose(&mut e, bear);
        if fizzle {
            move_to(&mut e, bear, ZoneLocation::Exile(P0));
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state()
                .object(liliana)
                .unwrap()
                .counters
                .get(CounterKind::P1P1),
            u16::from(!fizzle)
        );
        assert_eq!(library_size(&e, P0), library - if fizzle { 0 } else { 2 });
        assert!(!offered(&e, liliana, 1));
        reach_their_main_phase(&mut e, P1);
        reach_their_main_phase(&mut e, P0);
        e.dev_state_mut(P0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Black, 20);
        e.refresh_offer();
        assert!(!offered(&e, liliana, 1), "exhaust is not a per-turn limit");
        let hash = e.state().snapshot_hash();
        assert!(
            e.apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: liliana,
                    ability_index: 1
                }
            )
            .is_err()
        );
        assert_eq!(e.state().snapshot_hash(), hash);
        move_to(&mut e, liliana, ZoneLocation::Exile(P0));
        move_to(&mut e, liliana, ZoneLocation::Battlefield);
        pass_until(&mut e, stack_is_empty);
        assert!(offered(&e, liliana, 1));
        activate(&mut e, P0, index::LILIANA_THE_REPENTANT, 1);
        choose(&mut e, angel);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(angel).unwrap().zone, Zone::Battlefield);
    }
}

#[test]
fn defined_x_voodoo_pays_twice_current_pins_but_damage_uses_resolution_or_last_known_pins() {
    for leaves in [false, true] {
        let mut e = prepared(&[], &[index::VOODOO_DOLL], &[]);
        let doll = on_battlefield(&e, P0, index::VOODOO_DOLL).unwrap();
        e.dev_state_mut(P0)
            .unwrap()
            .object_mut(doll)
            .unwrap()
            .counters
            .set(counters::PIN, 3);
        let before = e.state().players[0].mana_pool.total();
        e.refresh_offer();
        activate(&mut e, P0, index::VOODOO_DOLL, 2);
        assert!(
            matches!(e.pending(), Pending::ChooseTargets { .. }),
            "defined X is never a free numeric choice"
        );
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
        assert_eq!(e.state().players[0].mana_pool.total(), before - 6);
        e.dev_state_mut(P0)
            .unwrap()
            .object_mut(doll)
            .unwrap()
            .counters
            .set(counters::PIN, 5);
        if leaves {
            move_to(&mut e, doll, ZoneLocation::Graveyard(P0));
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().players[1].life, 15);
    }
}

#[test]
fn defined_x_voodoo_end_step_destroys_then_deals_last_known_damage_and_rechecks_untapped() {
    for tap_in_response in [false, true] {
        let mut e = prepared(&[], &[index::VOODOO_DOLL], &[]);
        let doll = on_battlefield(&e, P0, index::VOODOO_DOLL).unwrap();
        // The first upkeep already placed the first pin.
        assert_eq!(
            e.state().object(doll).unwrap().counters.get(counters::PIN),
            1
        );
        pass_until(&mut e, |e| {
            e.state().turn.step == Step::End && !stack_is_empty(e)
        });
        if tap_in_response {
            e.dev_state_mut(P0).unwrap().set_tapped(doll, true);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(doll).unwrap().zone,
            if tap_in_response {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
        assert_eq!(
            e.state().players[0].life,
            if tap_in_response { 20 } else { 19 }
        );
    }
}

#[test]
fn minimum_x_lair_may_tap_itself_for_mana_while_paying_its_animation() {
    let mut e = prepared(&[], &[index::LAIR_OF_THE_HYDRA], &[]);
    let lair = on_battlefield(&e, P0, index::LAIR_OF_THE_HYDRA).unwrap();
    let state = e.dev_state_mut(P0).unwrap();
    state.players[0].mana_pool = baylee_core::mana::ManaPool::new();
    state.players[0].mana_pool.add(ManaColor::Colorless, 1);
    e.refresh_offer();
    activate(&mut e, P0, index::LAIR_OF_THE_HYDRA, 1);
    assert!(matches!(e.pending(), Pending::ChooseNumber { min: 1, .. }));
    e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    activate(&mut e, P0, index::LAIR_OF_THE_HYDRA, 0);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        e.state().object(lair).unwrap().characteristics().power,
        Some(1)
    );
    assert!(is_tapped(&e, lair));
}
