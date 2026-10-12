//! A source-bound cast trigger differs from a delayed mana-spend trigger.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::Cause;

#[test]
fn batch46_pools_copies_actual_announced_x_and_preserves_cast_decisions_after_countering() {
    for counter_original in [false, true] {
        let mut e = Duel::new(SEED, index::FOREST)
            .hand(0, &[index::WALKING_BALLISTA])
            .hand(1, &[index::COUNTERSPELL])
            .battlefield(0, &[index::THE_EVERFLOWING_WELL, index::SOL_RING])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        pools(&mut e);
        let ring = on_battlefield(&e, P0, index::SOL_RING).unwrap();
        e.state
            .object_mut(ring)
            .unwrap()
            .counters
            .add(CounterKind::P1P1, 1);
        reset_pool(&mut e);
        activate(&mut e, P0, index::THE_EVERFLOWING_WELL, 0);
        e.state.players[0].mana_pool.add(ManaColor::Colorless, 5);
        e.state.players[1].mana_pool.add(ManaColor::Blue, 2);
        let spell = in_hand(&e, P0, index::WALKING_BALLISTA).unwrap();
        cast(&mut e, spell);
        e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
        assert_eq!(e.state.object(spell).unwrap().x_value, 3);
        assert_eq!(e.state.players[0].mana_pool.total(), 0);
        // No registered permanent currently has kicker. Supply only the
        // independent decision bit on the real, paid-for permanent spell;
        // this exercises copying that bit, not kicker affordability.
        e.state.object_mut(spell).unwrap().kicked = true;
        target(&mut e, vec![ring], vec![]);
        if counter_original {
            e.apply(P0, PlayerAction::PassPriority).unwrap();
            let counter = in_hand(&e, P1, index::COUNTERSPELL).unwrap();
            e.apply(P1, PlayerAction::CastSpell { card: counter })
                .unwrap();
            target(&mut e, vec![spell], vec![]);
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.object(ring).unwrap().x_value, 3);
        assert!(e.state.object(ring).unwrap().kicked);
        assert_eq!(
            e.state.object(ring).unwrap().characteristics().mana_value(),
            0
        );
        assert_eq!(
            pt(&e, ring),
            (1, 1),
            "copying a spell does not apply its enter-with-X replacement"
        );
        assert_eq!(counters_on(&e, ring, CounterKind::P1P1), 1);
    }
}

fn pools(e: &mut Engine<RegistryLookup>) -> ObjectId {
    let source = on_battlefield(e, P0, index::THE_EVERFLOWING_WELL).unwrap();
    e.state.switch_face(
        source,
        RegistryLookup.card(index::THE_EVERFLOWING_WELL).unwrap(),
        1,
    );
    e.sync_static_effects();
    e.refresh_offer();
    source
}

#[test]
fn batch46_pools_copies_once_for_two_spent_units_and_reverts_at_cleanup() {
    let mut e = prepared(
        &[index::WORN_POWERSTONE],
        &[index::THE_EVERFLOWING_WELL, index::SOL_RING],
        &[],
    );
    let source = pools(&mut e);
    let ring = on_battlefield(&e, P0, index::SOL_RING).unwrap();
    // A counter belongs to the permanent, not the copied characteristics.
    e.state
        .object_mut(ring)
        .unwrap()
        .counters
        .add(CounterKind::P1P1, 1);
    reset_pool(&mut e);
    activate(&mut e, P0, index::THE_EVERFLOWING_WELL, 0);
    e.state.set_tapped(source, false);
    e.refresh_offer();
    activate(&mut e, P0, index::THE_EVERFLOWING_WELL, 0);
    let spell = in_hand(&e, P0, index::WORN_POWERSTONE).unwrap();
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 1);
    let copied_name = e.state.object(spell).unwrap().base.name;
    let original_name = e.state.object(ring).unwrap().base.name;
    cast(&mut e, spell);
    target(&mut e, vec![ring], vec![]);
    pass_until(&mut e, stack_is_empty);
    let copied = crate::layers::copiable_values(e.state(), ring).unwrap();
    assert_eq!(copied.name, copied_name);
    assert_eq!(
        e.state
            .object(ring)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    let turn = e.state.turn.number;
    pass_until(&mut e, |e| e.state.turn.number > turn);
    assert_eq!(
        crate::layers::copiable_values(e.state(), ring)
            .unwrap()
            .name,
        original_name
    );
}

#[test]
fn batch46_pools_leaving_before_cast_prevents_its_trigger() {
    let mut e = prepared(&[index::SOL_RING], &[index::THE_EVERFLOWING_WELL], &[]);
    let source = pools(&mut e);
    reset_pool(&mut e);
    activate(&mut e, P0, index::THE_EVERFLOWING_WELL, 0);
    e.state
        .move_object(
            source,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let spell = in_hand(&e, P0, index::SOL_RING).unwrap();
    cast(&mut e, spell);
    assert_eq!(e.state.zones.list(ZoneLocation::Stack).len(), 1);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::SOL_RING).is_some());
}

#[test]
fn batch46_pools_target_is_optional_and_excludes_itself_and_opponents() {
    let mut e = prepared(
        &[index::SOL_RING],
        &[index::THE_EVERFLOWING_WELL, index::FOREST],
        &[index::MOUNTAIN],
    );
    let source = pools(&mut e);
    reset_pool(&mut e);
    activate(&mut e, P0, index::THE_EVERFLOWING_WELL, 0);
    let spell = in_hand(&e, P0, index::SOL_RING).unwrap();
    cast(&mut e, spell);
    assert!(matches!(e.pending(), Pending::ChooseTargets { min: 0, .. }));
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![source],
                players: vec![]
            }
        )
        .is_err()
    );
    let opponent = on_battlefield(&e, P1, index::MOUNTAIN).unwrap();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![opponent],
                players: vec![]
            }
        )
        .is_err()
    );
    target(&mut e, vec![], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::SOL_RING).is_some());
}

#[test]
fn batch46_well_descend_counts_eight_permanent_cards_and_rechecks_on_resolution() {
    for (count, remove) in [(7, false), (8, false), (8, true)] {
        let mut e = prepared(&[], &[index::THE_EVERFLOWING_WELL], &[]);
        let well = on_battlefield(&e, P0, index::THE_EVERFLOWING_WELL).unwrap();
        seed_graveyard(&mut e, P0, count);
        pass_until(&mut e, |e| {
            e.state.turn.active == P0 && e.state.turn.step == crate::turn::Step::Upkeep
        });
        if remove {
            let card = e.state.zones.list(ZoneLocation::Graveyard(P0))[0];
            e.state
                .move_object(
                    card,
                    ZoneLocation::Hand(P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state.object(well).unwrap().face_index,
            u8::from(count == 8 && !remove)
        );
    }
}

#[test]
fn batch46_pools_recognizes_mana_produced_before_it_gained_the_ability() {
    let mut e = prepared(
        &[index::WORN_POWERSTONE],
        &[index::THE_EVERFLOWING_WELL, index::SOL_RING, index::FOREST],
        &[],
    );
    let original = pools(&mut e);
    let source = on_battlefield(&e, P0, index::SOL_RING).unwrap();
    let target_card = on_battlefield(&e, P0, index::FOREST).unwrap();
    reset_pool(&mut e);
    activate(&mut e, P0, index::SOL_RING, 0);
    assert!(crate::copiable_abilities::apply_copy(
        &mut e.state,
        crate::copiable_abilities::CopyApplication {
            source,
            target: original,
            mods: &[],
            copy_index: 0,
            until_eot: true,
            resolving: None,
            text: crate::text_changes::TextChangeMap::IDENTITY,
        }
    ));
    // Keep only one legendary Pools before the next state-based action.
    e.state
        .move_object(
            original,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.sync_static_effects();
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 1);
    let spell = in_hand(&e, P0, index::WORN_POWERSTONE).unwrap();
    let name = e.state.object(spell).unwrap().base.name;
    cast(&mut e, spell);
    target(&mut e, vec![target_card], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        crate::layers::copiable_values(e.state(), target_card)
            .unwrap()
            .name,
        name
    );
}

#[test]
fn batch46_pools_copies_the_exact_spell_after_it_is_countered() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, &[index::WORN_POWERSTONE])
        .hand(1, &[index::COUNTERSPELL])
        .battlefield(0, &[index::THE_EVERFLOWING_WELL, index::SOL_RING])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    pools(&mut e);
    activate(&mut e, P0, index::THE_EVERFLOWING_WELL, 0);
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 2);
    e.state.players[1].mana_pool.add(ManaColor::Blue, 2);
    let ring = on_battlefield(&e, P0, index::SOL_RING).unwrap();
    let spell = in_hand(&e, P0, index::WORN_POWERSTONE).unwrap();
    let name = e.state.object(spell).unwrap().base.name;
    cast(&mut e, spell);
    target(&mut e, vec![ring], vec![]);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    let counter = in_hand(&e, P1, index::COUNTERSPELL).unwrap();
    e.apply(P1, PlayerAction::CastSpell { card: counter })
        .unwrap();
    target(&mut e, vec![spell], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(spell).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert_eq!(
        crate::layers::copiable_values(e.state(), ring)
            .unwrap()
            .name,
        name
    );
}

#[test]
fn batch46_pools_cleanup_restores_a_previous_permanent_copy_and_its_activated_body() {
    let mut e = prepared(
        &[index::WALKING_BALLISTA],
        &[
            index::THE_EVERFLOWING_WELL,
            index::SOL_RING,
            index::JAYEMDAE_TOME,
        ],
        &[],
    );
    pools(&mut e);
    let ring = on_battlefield(&e, P0, index::SOL_RING).unwrap();
    let tome = on_battlefield(&e, P0, index::JAYEMDAE_TOME).unwrap();
    assert!(crate::copiable_abilities::apply_copy(
        &mut e.state,
        crate::copiable_abilities::CopyApplication {
            source: ring,
            target: tome,
            mods: &[],
            copy_index: 0,
            until_eot: false,
            resolving: None,
            text: crate::text_changes::TextChangeMap::IDENTITY,
        }
    ));
    e.sync_static_effects();
    e.state
        .object_mut(ring)
        .unwrap()
        .counters
        .add(CounterKind::P1P1, 1);
    let name = crate::layers::copiable_values(e.state(), ring)
        .unwrap()
        .name;
    reset_pool(&mut e);
    activate(&mut e, P0, index::THE_EVERFLOWING_WELL, 0);
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 5);
    let spell = in_hand(&e, P0, index::WALKING_BALLISTA).unwrap();
    cast(&mut e, spell);
    e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    target(&mut e, vec![ring], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, ring), (1, 1));
    assert_eq!(e.state.object(ring).unwrap().x_value, 3);
    let turn = e.state.turn.number;
    pass_until(&mut e, |e| e.state.turn.number > turn);
    assert_eq!(
        crate::layers::copiable_values(e.state(), ring)
            .unwrap()
            .name,
        name
    );
    assert_eq!(e.state.object(ring).unwrap().x_value, 0);
    assert_eq!(
        counters_on(&e, ring, CounterKind::P1P1),
        1,
        "cleanup preserves the recipient counter"
    );
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    reset_pool(&mut e);
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 4);
    e.refresh_offer();
    let before = e.state.zones.list(ZoneLocation::Hand(P0)).len();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: ring,
            ability_index: 0,
        },
    )
    .unwrap();
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    assert!(
        !stack_is_empty(&e),
        "the restored definition is Tome's draw, not Ring's mana"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.zones.list(ZoneLocation::Hand(P0)).len(), before + 1);
}

#[test]
fn batch46_pools_two_temporary_overlays_restore_the_pre_first_copy_rules_and_choices() {
    let mut e = prepared(
        &[index::WORN_POWERSTONE, index::WALKING_BALLISTA],
        &[index::THE_EVERFLOWING_WELL, index::SOL_RING],
        &[],
    );
    let source = pools(&mut e);
    let ring = on_battlefield(&e, P0, index::SOL_RING).unwrap();
    e.state
        .object_mut(ring)
        .unwrap()
        .counters
        .add(CounterKind::P1P1, 1);
    let original_name = e.state.object(ring).unwrap().base.name;
    reset_pool(&mut e);
    activate(&mut e, P0, index::THE_EVERFLOWING_WELL, 0);
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 2);
    let powerstone = in_hand(&e, P0, index::WORN_POWERSTONE).unwrap();
    let first_name = e.state.object(powerstone).unwrap().base.name;
    cast(&mut e, powerstone);
    target(&mut e, vec![ring], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        crate::layers::copiable_values(e.state(), ring)
            .unwrap()
            .name,
        first_name
    );
    e.state.set_tapped(source, false);
    reset_pool(&mut e);
    activate(&mut e, P0, index::THE_EVERFLOWING_WELL, 0);
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 5);
    let ballista = in_hand(&e, P0, index::WALKING_BALLISTA).unwrap();
    cast(&mut e, ballista);
    e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    target(&mut e, vec![ring], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(ring).unwrap().x_value, 3);
    assert_eq!(pt(&e, ring), (1, 1));
    let turn = e.state.turn.number;
    pass_until(&mut e, |e| e.state.turn.number > turn);
    assert_eq!(
        crate::layers::copiable_values(e.state(), ring)
            .unwrap()
            .name,
        original_name
    );
    assert_eq!(e.state.object(ring).unwrap().x_value, 0);
    assert_eq!(counters_on(&e, ring, CounterKind::P1P1), 1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    reset_pool(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: ring,
            ability_index: 0,
        },
    )
    .unwrap();
    assert!(stack_is_empty(&e));
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        2
    );
}

#[test]
fn batch46_pools_cleanup_does_not_restore_old_copy_rules_onto_a_returned_recipient() {
    let mut e = prepared(
        &[index::WALKING_BALLISTA],
        &[
            index::THE_EVERFLOWING_WELL,
            index::SOL_RING,
            index::JAYEMDAE_TOME,
        ],
        &[],
    );
    pools(&mut e);
    let ring = on_battlefield(&e, P0, index::SOL_RING).unwrap();
    let tome = on_battlefield(&e, P0, index::JAYEMDAE_TOME).unwrap();
    // Establish a prior permanent rules bundle, as in the restoration regression.
    assert!(crate::copiable_abilities::apply_copy(
        &mut e.state,
        crate::copiable_abilities::CopyApplication {
            source: ring,
            target: tome,
            mods: &[],
            copy_index: 0,
            until_eot: false,
            resolving: None,
            text: crate::text_changes::TextChangeMap::IDENTITY,
        }
    ));
    e.sync_static_effects();
    e.state
        .object_mut(ring)
        .unwrap()
        .counters
        .add(CounterKind::P1P1, 1);
    reset_pool(&mut e);
    activate(&mut e, P0, index::THE_EVERFLOWING_WELL, 0);
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 5);
    let ballista = in_hand(&e, P0, index::WALKING_BALLISTA).unwrap();
    cast(&mut e, ballista);
    e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    target(&mut e, vec![ring], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(ring).unwrap().x_value, 3);
    let old_version = e.state.object(ring).unwrap().version;
    e.state
        .move_object(
            ring,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            ring,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.sync_static_effects();
    assert_ne!(e.state.object(ring).unwrap().version, old_version);
    let turn = e.state.turn.number;
    pass_until(&mut e, |e| e.state.turn.number > turn);
    assert_eq!(e.state.object(ring).unwrap().x_value, 0);
    assert_eq!(counters_on(&e, ring, CounterKind::P1P1), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    reset_pool(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: ring,
            ability_index: 0,
        },
    )
    .unwrap();
    assert!(
        stack_is_empty(&e),
        "new Sol Ring must not acquire old incarnation's Tome draw"
    );
    assert_eq!(
        e.state.players[0].mana_pool.available(ManaColor::Colorless),
        2
    );
}
