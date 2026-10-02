//! Sengir Vampire's real combat, death triggers and later removal spells.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::GameEvent;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn setup(defenders: &[CardIndex], hand: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = Duel::new(1150, forest())
        .battlefield(0, &[sengir_vampire()])
        .battlefield(1, defenders)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let vampire = on_battlefield(&e, P0, sengir_vampire()).unwrap();
    (e, vampire)
}

fn block(e: &mut Engine<RegistryLookup>, vampire: ObjectId, blockers: &[ObjectId]) {
    let offered = attack_and_collect_blocks(e, vampire, P1);
    for blocker in blockers {
        assert!(
            offered
                .iter()
                .any(|b| b.blocker == *blocker && b.attackers.contains(&vampire))
        );
    }
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: blockers.iter().map(|b| (*b, vampire)).collect(),
        },
    )
    .unwrap();
}

fn triggers(e: &Engine<RegistryLookup>, vampire: ObjectId) -> usize {
    e.journal()
        .entries()
        .iter()
        .filter(|entry| {
            matches!(entry.event,
                GameEvent::AbilityTriggered { source, .. } if source == vampire
            )
        })
        .count()
}

fn target_spell(e: &mut Engine<RegistryLookup>, spell: CardIndex, target: ObjectId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    let state = e.dev_state_mut(P0).unwrap();
    for color in [
        ManaColor::Black,
        ManaColor::Blue,
        ManaColor::White,
        ManaColor::Colorless,
    ] {
        state.players[0].mana_pool.add(color, 4);
    }
    e.refresh_offer();
    cast_with_floating(e, P0, spell);
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![target],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(e, stack_is_empty);
}

#[test]
fn sengir_vampire_combat_kill_adds_one_counter_after_damage() {
    let (mut e, vampire) = setup(&[index::STORM_CROW], &[]);
    let crow = on_battlefield(&e, P1, index::STORM_CROW).unwrap();
    block(&mut e, vampire, &[crow]);
    pass_until(&mut e, |e| {
        e.state().object(crow).unwrap().zone == Zone::Graveyard
    });
    assert_eq!(triggers(&e, vampire), 1);
    assert_eq!(
        counters_on(&e, vampire, CounterKind::P1P1),
        0,
        "trigger must resolve first"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, vampire, CounterKind::P1P1), 1);
    assert_eq!(pt(&e, vampire), (5, 5));
}

#[test]
fn sengir_vampire_gets_one_counter_for_each_damaged_blocker_that_dies() {
    let (mut e, vampire) = setup(&[index::STORM_CROW, index::WIND_DRAKE], &[]);
    let crow = on_battlefield(&e, P1, index::STORM_CROW).unwrap();
    let drake = on_battlefield(&e, P1, index::WIND_DRAKE).unwrap();
    block(&mut e, vampire, &[crow, drake]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    assert!(matches!(e.pending(), Pending::ChooseNumber {
        player, reason: crate::choice::NumberPrompt::CombatDamage { source, .. }, ..
    } if *player == P0 && *source == vampire));
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(triggers(&e, vampire), 2);
    assert_eq!(counters_on(&e, vampire, CounterKind::P1P1), 2);
    assert_eq!(pt(&e, vampire), (6, 6));
}

#[test]
fn sengir_vampire_nonlethal_combat_then_terror_still_grows() {
    let (mut e, vampire) = setup(&[index::WALL_OF_SWORDS], &[index::TERROR]);
    let wall = on_battlefield(&e, P1, index::WALL_OF_SWORDS).unwrap();
    block(&mut e, vampire, &[wall]);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(e.state().object(wall).unwrap().damage, 4);
    assert_eq!(triggers(&e, vampire), 0);
    target_spell(&mut e, index::TERROR, wall);
    assert_eq!(triggers(&e, vampire), 1);
    assert_eq!(pt(&e, vampire), (5, 5));
}

#[test]
fn sengir_vampire_does_not_grow_for_an_undamaged_creatures_death() {
    let (mut e, vampire) = setup(
        &[index::WALL_OF_SWORDS, index::LLANOWAR_ELVES],
        &[index::TERROR],
    );
    let wall = on_battlefield(&e, P1, index::WALL_OF_SWORDS).unwrap();
    let elf = on_battlefield(&e, P1, index::LLANOWAR_ELVES).unwrap();
    block(&mut e, vampire, &[wall]);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    target_spell(&mut e, index::TERROR, elf);
    assert_eq!(triggers(&e, vampire), 0);
    assert_eq!(pt(&e, vampire), (4, 4));
}

#[test]
fn sengir_vampire_does_not_remember_damage_from_an_earlier_turn() {
    let (mut e, vampire) = setup(&[index::WALL_OF_SWORDS], &[index::TERROR]);
    let wall = on_battlefield(&e, P1, index::WALL_OF_SWORDS).unwrap();
    block(&mut e, vampire, &[wall]);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    reach_their_main_phase(&mut e, P1);
    assert_eq!(e.state().object(wall).unwrap().damage, 0);
    target_spell(&mut e, index::TERROR, wall);
    assert_eq!(triggers(&e, vampire), 0);
    assert_eq!(pt(&e, vampire), (4, 4));
}

#[test]
fn sengir_vampire_trades_simultaneously_and_triggers_without_receiving_a_counter() {
    let (mut e, vampire) = setup(&[index::AIR_ELEMENTAL], &[]);
    let elemental = on_battlefield(&e, P1, index::AIR_ELEMENTAL).unwrap();
    block(&mut e, vampire, &[elemental]);
    pass_until(&mut e, |e| {
        e.state().object(elemental).unwrap().zone == Zone::Graveyard
    });
    assert_eq!(e.state().object(vampire).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        triggers(&e, vampire),
        1,
        "simultaneous death still triggers"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, vampire, CounterKind::P1P1), 0);
}

#[test]
fn sengir_vampire_bounced_in_response_does_not_receive_a_counter_in_hand() {
    let (mut e, vampire) = setup(&[index::STORM_CROW], &[index::UNSUMMON]);
    let crow = on_battlefield(&e, P1, index::STORM_CROW).unwrap();
    block(&mut e, vampire, &[crow]);
    pass_until(&mut e, |e| {
        e.state().object(crow).unwrap().zone == Zone::Graveyard
    });
    assert_eq!(triggers(&e, vampire), 1);
    target_spell(&mut e, index::UNSUMMON, vampire);
    assert_eq!(e.state().object(vampire).unwrap().zone, Zone::Hand);
    assert_eq!(counters_on(&e, vampire, CounterKind::P1P1), 0);
}

#[test]
fn sengir_vampire_damaged_creature_exiled_instead_of_dying_does_not_trigger() {
    let (mut e, vampire) = setup(&[index::WALL_OF_SWORDS], &[index::SWORDS_TO_PLOWSHARES]);
    let wall = on_battlefield(&e, P1, index::WALL_OF_SWORDS).unwrap();
    block(&mut e, vampire, &[wall]);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    target_spell(&mut e, index::SWORDS_TO_PLOWSHARES, wall);
    assert_eq!(e.state().object(wall).unwrap().zone, Zone::Exile);
    assert_eq!(triggers(&e, vampire), 0);
}

#[test]
fn sengir_vampire_fogged_combat_then_death_does_not_trigger() {
    let (mut e, vampire) = setup(&[index::STORM_CROW], &[index::FOG, index::TERROR]);
    let crow = on_battlefield(&e, P1, index::STORM_CROW).unwrap();
    block(&mut e, vampire, &[crow]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    e.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 1);
    e.refresh_offer();
    cast_with_floating(&mut e, P0, index::FOG);
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(e.state().object(crow).unwrap().damage, 0);
    target_spell(&mut e, index::TERROR, crow);
    assert_eq!(triggers(&e, vampire), 0);
    assert_eq!(pt(&e, vampire), (4, 4));
}

#[test]
fn sengir_vampire_casts_for_three_generic_and_two_black_as_a_four_four_flier() {
    let mut e = Duel::new(1151, forest())
        .battlefield(0, &[swamp(), swamp(), forest(), forest(), forest()])
        .hand(0, &[sengir_vampire()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast_from_hand(&mut e, P0, sengir_vampire());
    pass_until(&mut e, stack_is_empty);
    let vampire = on_battlefield(&e, P0, sengir_vampire()).unwrap();
    assert_eq!(pt(&e, vampire), (4, 4));
    assert!(keywords(&e, vampire).contains(KeywordSet::FLYING));
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

#[test]
fn sengir_vampire_noncombat_fights_trigger_once_per_death_not_per_hit() {
    for (victim_card, hits) in [(index::LLANOWAR_ELVES, 1), (index::WALL_OF_AIR, 2)] {
        let (mut e, vampire) = setup(
            &[victim_card],
            &[index::KHALNI_AMBUSH, index::KHALNI_AMBUSH],
        );
        let victim = on_battlefield(&e, P1, victim_card).unwrap();
        for hit in 1..=hits {
            pass_until(
                &mut e,
                |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
            );
            e.dev_state_mut(P0).unwrap().players[0]
                .mana_pool
                .add(ManaColor::Green, 3);
            e.refresh_offer();
            cast_front_face(&mut e, P0, index::KHALNI_AMBUSH);
            for target in [vampire, victim] {
                e.apply(
                    P0,
                    PlayerAction::ChooseTargets {
                        objects: vec![target],
                        players: vec![],
                    },
                )
                .unwrap();
            }
            pass_until(&mut e, stack_is_empty);
            assert_eq!(triggers(&e, vampire), usize::from(hit == hits));
        }
        assert_eq!(e.state().object(victim).unwrap().zone, Zone::Graveyard);
        assert_eq!(counters_on(&e, vampire, CounterKind::P1P1), 1);
        assert_eq!(pt(&e, vampire), (5, 5));
    }
}
