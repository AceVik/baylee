//! Exact death objects, granted return abilities and changing reanimation.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::Cause;
use crate::zone::ZonePosition;
use baylee_core::generated::{index, subtypes};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn advance(e: &mut Engine<RegistryLookup>) {
    let Pending::Priority { player, .. } = e.pending().clone() else {
        panic!("priority: {:?}", e.pending())
    };
    e.apply(player, PlayerAction::PassPriority).unwrap();
}

#[test]
fn granted_undying_survives_simultaneous_death_but_not_prior_loss() {
    for simultaneous in [false, true] {
        let mut e = Duel::new(SEED, swamp())
            .battlefield(0, &[index::MIKAEUS_THE_UNHALLOWED, index::LLANOWAR_ELVES])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        let source = on_battlefield(&e, P0, index::MIKAEUS_THE_UNHALLOWED).unwrap();
        let elf = on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap();
        let state = e.dev_state_mut(P0).unwrap();
        if simultaneous {
            crate::sba::destroy_all(state, &[source, elf], false);
        } else {
            crate::sba::destroy(state, source);
            state.refresh_characteristics();
            crate::sba::destroy(state, elf);
        }
        advance(&mut e);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            on_battlefield(&e, P0, index::LLANOWAR_ELVES).is_some(),
            simultaneous
        );
    }
}

#[test]
fn printed_and_granted_undying_trigger_independently_but_return_only_once() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[index::MIKAEUS_THE_UNHALLOWED, index::YOUNG_WOLF])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let wolf = on_battlefield(&e, P0, index::YOUNG_WOLF).unwrap();
    crate::sba::destroy(e.dev_state_mut(P0).unwrap(), wolf);
    advance(&mut e);
    assert_eq!(e.state().zones.list(ZoneLocation::Stack).len(), 2);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, wolf, CounterKind::P1P1), 1);
    assert_eq!(all_on_battlefield(&e, P0, index::YOUNG_WOLF), vec![wolf]);
}

#[test]
fn mikaeus_retaliates_against_human_damage_including_noncombat() {
    for combat in [false, true] {
        let mut e = Duel::new(SEED, swamp())
            .battlefield(0, &[index::MIKAEUS_THE_UNHALLOWED])
            .battlefield(1, &[index::ROYAL_ASSASSIN, index::LLANOWAR_ELVES])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        let human = on_battlefield(&e, P1, index::ROYAL_ASSASSIN).unwrap();
        let elf = on_battlefield(&e, P1, index::LLANOWAR_ELVES).unwrap();
        let state = e.dev_state_mut(P0).unwrap();
        state.damage_player(human, P0, 1, combat, Cause::Effect);
        state.damage_player(elf, P0, 1, combat, Cause::Effect);
        advance(&mut e);
        pass_until(&mut e, stack_is_empty);
        assert!(on_battlefield(&e, P1, index::ROYAL_ASSASSIN).is_none());
        assert!(on_battlefield(&e, P1, index::LLANOWAR_ELVES).is_some());
    }
}

#[test]
fn deathmantle_returns_its_death_card_then_attaches_if_still_present() {
    for (equipment_stays, same_card) in [(true, true), (false, true), (true, false)] {
        let mut e = Duel::new(SEED, swamp())
            .battlefield(
                0,
                &[
                    index::NIM_DEATHMANTLE,
                    index::LLANOWAR_ELVES,
                    swamp(),
                    swamp(),
                    swamp(),
                    swamp(),
                ],
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        let mantle = on_battlefield(&e, P0, index::NIM_DEATHMANTLE).unwrap();
        let elf = on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap();
        tap_all_mana_but(&mut e, P0, Some(index::LLANOWAR_ELVES));
        crate::sba::destroy(e.dev_state_mut(P0).unwrap(), elf);
        advance(&mut e);
        let state = e.dev_state_mut(P0).unwrap();
        if !equipment_stays {
            crate::sba::destroy(state, mantle);
        }
        if !same_card {
            state
                .move_object(
                    elf,
                    ZoneLocation::Exile(P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
            state
                .move_object(
                    elf,
                    ZoneLocation::Graveyard(P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        e.apply(P0, PlayerAction::YesNo(true)).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            on_battlefield(&e, P0, index::LLANOWAR_ELVES).is_some(),
            same_card
        );
        assert_eq!(
            e.state().object(mantle).unwrap().attached_to,
            (equipment_stays && same_card).then_some(elf)
        );
        assert_eq!(e.state().players[0].mana_pool.total(), 0);
    }
}

#[test]
fn assimilation_replaces_creature_types_doubles_entry_counters_and_outlives_queen() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                index::DOUBLING_SEASON,
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[index::BORG_QUEEN_PERFECTION_MANIFEST])
        .hand(1, &[index::LLANOWAR_ELVES])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let elf = in_hand(&e, P1, index::LLANOWAR_ELVES).unwrap();
    e.dev_state_mut(P0)
        .unwrap()
        .move_object(
            elf,
            ZoneLocation::Graveyard(P1),
            ZonePosition::Top,
            Cause::Setup,
        )
        .unwrap();
    tap_all_mana_but(&mut e, P0, None);
    cast_with_floating(&mut e, P0, index::BORG_QUEEN_PERFECTION_MANIFEST);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    let object = e.state().object(elf).unwrap();
    assert_eq!(object.controller, P0);
    assert_eq!(object.owner, P1);
    assert!(object.characteristics().types.contains(TypeSet::ARTIFACT));
    assert!(
        object
            .characteristics()
            .subtypes
            .contains(subtypes::creature::BORG)
    );
    assert!(
        !object
            .characteristics()
            .subtypes
            .contains(subtypes::creature::ELF)
    );
    assert!(
        !object
            .characteristics()
            .subtypes
            .contains(subtypes::creature::DRUID)
    );
    assert_eq!(counters_on(&e, elf, CounterKind::P1P1), 2);
    assert_eq!(pt(&e, elf), (5, 3));
    let queen = on_battlefield(&e, P0, index::BORG_QUEEN_PERFECTION_MANIFEST).unwrap();
    crate::sba::destroy(e.dev_state_mut(P0).unwrap(), queen);
    advance(&mut e);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, elf), (3, 3));
    assert!(
        e.state()
            .object(elf)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::BORG)
    );
}

#[test]
fn retaliation_cannot_destroy_a_later_incarnation_of_the_damaging_human() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[index::MIKAEUS_THE_UNHALLOWED])
        .battlefield(1, &[index::ROYAL_ASSASSIN])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let human = on_battlefield(&e, P1, index::ROYAL_ASSASSIN).unwrap();
    e.dev_state_mut(P0)
        .unwrap()
        .damage_player(human, P0, 1, false, Cause::Effect);
    advance(&mut e);
    let state = e.dev_state_mut(P0).unwrap();
    state
        .move_object(
            human,
            ZoneLocation::Exile(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            human,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P1, index::ROYAL_ASSASSIN).is_some());
}

#[test]
fn removing_granted_undying_before_death_prevents_the_return() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[index::MIKAEUS_THE_UNHALLOWED, index::LLANOWAR_ELVES])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let elf = on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap();
    let state = e.dev_state_mut(P0).unwrap();
    super::keyword_counters::remove_abilities(state, elf);
    state.refresh_characteristics();
    crate::sba::destroy(state, elf);
    advance(&mut e);
    assert!(stack_is_empty(&e));
    assert!(in_graveyard(&e, P0, index::LLANOWAR_ELVES).is_some());
}

#[test]
fn an_undying_token_triggers_but_cannot_return_from_the_graveyard() {
    let mut e = Duel::new(SEED, swamp()).start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let state = e.dev_state_mut(P0).unwrap();
    let name = state.names.intern("Undying token");
    let token = state.create_bare(
        P0,
        crate::object::ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let base = state.object_mut(token).unwrap().base_mut();
    base.types = TypeSet::CREATURE;
    base.power = Some(2);
    base.toughness = Some(2);
    base.keywords = KeywordSet::UNDYING;
    state.invalidate_projections();
    state.refresh_characteristics();
    crate::sba::destroy(state, token);
    advance(&mut e);
    assert_eq!(e.state().zones.list(ZoneLocation::Stack).len(), 1);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .object(token)
            .is_none_or(|o| o.zone != crate::zone::Zone::Battlefield)
    );
}
