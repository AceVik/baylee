//! Historical entry facts and real card actions.
#[allow(clippy::wildcard_imports)]
use super::*;

fn surveil_now(e: &mut Engine<RegistryLookup>) {
    priority(e, P0);
    e.state.players[0].mana_pool.add(ManaColor::Blue, 1);
    e.refresh_offer();
    let village = source(e, P0, index::LILYPAD_VILLAGE);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: village,
            ability_index: 2,
        },
    )
    .unwrap();
    pass_until(e, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange { cards, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(cards.len(), 2);
    e.apply(
        P0,
        PlayerAction::Arrange {
            piles: vec![cards, vec![]],
        },
    )
    .unwrap();
    pass_until(e, stack_is_empty);
}

fn ritual(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    priority(e, player);
    e.state.players[usize::from(player.get())]
        .mana_pool
        .add(ManaColor::Black, 1);
    e.refresh_offer();
    let card = in_hand(e, player, index::DARK_RITUAL).unwrap();
    e.apply(player, PlayerAction::CastSpell { card }).unwrap();
    pass_until(e, stack_is_empty);
}

#[test]
fn druid_ignores_teammate_instants_but_tracks_each_actual_opponent() {
    let p2 = PlayerId::new(2);
    let mut e = Duel::table(SEED, index::FOREST, 3)
        .team(0, 1)
        .team(1, 1)
        .team(2, 2)
        .battlefield(0, &[index::ICHNEUMON_DRUID])
        .hand(1, &[index::DARK_RITUAL, index::DARK_RITUAL])
        .hand(2, &[index::DARK_RITUAL, index::DARK_RITUAL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let before = [e.state.players[1].life, e.state.players[2].life];
    ritual(&mut e, P1);
    ritual(&mut e, P1);
    assert_eq!(e.state.players[1].life, before[0]);
    ritual(&mut e, p2);
    assert_eq!(e.state.players[2].life, before[1]);
    ritual(&mut e, p2);
    assert_eq!(e.state.players[2].life, before[1] - 4);
}

#[test]
fn druid_counts_fork_cast_once_and_does_not_count_its_uncast_spell_copy() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::ICHNEUMON_DRUID])
        .hand(0, &[index::DARK_RITUAL])
        .hand(1, &[index::FORK, index::DARK_RITUAL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast(&mut e, index::DARK_RITUAL);
    let original = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    priority(&mut e, P1);
    e.state.players[1].mana_pool.add(ManaColor::Red, 2);
    e.refresh_offer();
    let fork = in_hand(&e, P1, index::FORK).unwrap();
    let life = e.state.players[1].life;
    e.apply(P1, PlayerAction::CastSpell { card: fork }).unwrap();
    targets(&mut e, vec![original], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.players[1].life, life,
        "Fork is first cast; copied Ritual was not cast"
    );
    ritual(&mut e, P1);
    assert_eq!(
        e.state.players[1].life,
        life - 4,
        "actual second cast triggers"
    );
}

#[test]
fn lilypad_counts_simultaneous_nexus_and_creature_return_before_nexus_disappears() {
    let mut e = prepared(
        &[index::EERIE_INTERLUDE, index::DISENCHANT],
        &[
            index::LILYPAD_VILLAGE,
            index::MASKWOOD_NEXUS,
            index::GRIZZLY_BEARS,
        ],
        &[],
    );
    let nexus = source(&e, P0, index::MASKWOOD_NEXUS);
    let bear = source(&e, P0, index::GRIZZLY_BEARS);
    // Interlude targets creatures only. Include Nexus via a transparent type
    // setup, then remove that extra type in exile before simultaneous return.
    let original_nexus = e.state.object(nexus).unwrap().base.clone();
    e.state.object_mut(nexus).unwrap().base_mut().types =
        baylee_core::types::TypeSet::ARTIFACT.union(baylee_core::types::TypeSet::CREATURE);
    e.state.object_mut(nexus).unwrap().base_mut().power = Some(1);
    e.state.object_mut(nexus).unwrap().base_mut().toughness = Some(1);
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    cast(&mut e, index::EERIE_INTERLUDE);
    targets(&mut e, vec![nexus, bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(nexus).unwrap().zone, Zone::Exile);
    // Explicitly restore the synthetic setup in exile (base_mut does not
    // automatically install an original_base copy rider).
    e.state.object_mut(nexus).unwrap().base = original_nexus;
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    pass_until(&mut e, |e| {
        e.state
            .object(nexus)
            .is_some_and(|o| o.zone == Zone::Battlefield)
            && e.state
                .object(bear)
                .is_some_and(|o| o.zone == Zone::Battlefield)
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { .. })
    });
    cast(&mut e, index::DISENCHANT);
    targets(&mut e, vec![nexus], vec![]);
    pass_until(&mut e, stack_is_empty);
    surveil_now(&mut e);
}

#[test]
fn lilypad_accepts_noncreature_kindred_subtype_entry_fixture() {
    let mut e = prepared(&[index::CRIB_SWAP], &[index::LILYPAD_VILLAGE], &[]);
    let card = in_hand(&e, P0, index::CRIB_SWAP).unwrap();
    // Isolated kindred-permanent entry: no registered kindred enchantment
    // supplies this precise case. Preserve Crib Swap's native changeling;
    // only replace instant type with enchantment before the fixture move.
    e.state.object_mut(card).unwrap().base_mut().types =
        baylee_core::types::TypeSet::KINDRED.union(baylee_core::types::TypeSet::ENCHANTMENT);
    e.state
        .move_object(
            card,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    assert!(
        !e.state
            .object(card)
            .unwrap()
            .characteristics()
            .types
            .contains(baylee_core::types::TypeSet::CREATURE)
    );
    surveil_now(&mut e);
}

#[test]
fn lilypad_retains_entry_and_departure_before_the_next_priority_capture_fixture() {
    let mut e = prepared(&[index::SEWER_RATS], &[index::LILYPAD_VILLAGE], &[]);
    let rat = in_hand(&e, P0, index::SEWER_RATS).unwrap();
    // Two ordinary zone events with no engine action/priority boundary between
    // them isolate a single-resolution entry/departure journal interval.
    e.state
        .move_object(
            rat,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            rat,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    surveil_now(&mut e);
}

#[test]
fn lilypad_does_not_treat_same_incarnation_phasing_in_as_an_entry() {
    let mut e = prepared(&[], &[index::LILYPAD_VILLAGE, index::SEWER_RATS], &[]);
    let rat = source(&e, P0, index::SEWER_RATS);
    let version = e.state.object(rat).unwrap().version;
    // Transparent phasing-only fixture: no zone change, no entry event.
    e.state
        .object_mut(rat)
        .unwrap()
        .status
        .insert(Status::PHASED_OUT);
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    e.state
        .object_mut(rat)
        .unwrap()
        .status
        .remove(Status::PHASED_OUT);
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    assert_eq!(e.state.object(rat).unwrap().version, version);
    let village = source(&e, P0, index::LILYPAD_VILLAGE);
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: village,
                ability_index: 2
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
}
