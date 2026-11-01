//! Arcum's Weathervane's two targeted, indefinite Snow changes, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn arcum_s_weathervane() -> CardIndex {
    card_index("b4e57fba-5fa3-4fd5-8f0a-7b61593ade72")
}

fn prepared() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[arcum_s_weathervane()])
        .battlefield(0, &[forest(), forest()])
        .battlefield(
            1,
            &[
                forest(),
                index::SNOW_COVERED_FOREST,
                index::SNOWFIELD_SINKHOLE,
                index::SOL_RING,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, arcum_s_weathervane());
    pass_until(&mut engine, stack_is_empty);
    engine
}

fn pay_two(engine: &mut Engine<RegistryLookup>) {
    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 2);
    engine.refresh_offer();
}

#[test]
fn weathervane_removes_snow_from_basic_and_nonbasic_lands_indefinitely() {
    for land in [index::SNOW_COVERED_FOREST, index::SNOWFIELD_SINKHOLE] {
        let mut engine = prepared();
        let vane = on_battlefield(&engine, P0, arcum_s_weathervane()).unwrap();
        let target = on_battlefield(&engine, P1, land).unwrap();
        let before = engine
            .state()
            .object(target)
            .unwrap()
            .characteristics()
            .clone();
        pay_two(&mut engine);
        activate(&mut engine, P0, arcum_s_weathervane(), 0);
        let options = aim_at(&mut engine, P0, target);
        assert_eq!(options.len(), 2, "both snow lands, regardless of Basic");
        for snow in [index::SNOW_COVERED_FOREST, index::SNOWFIELD_SINKHOLE] {
            assert!(options.contains(&on_battlefield(&engine, P1, snow).unwrap()));
        }
        assert!(
            engine
                .state()
                .object(vane)
                .unwrap()
                .status
                .contains(Status::TAPPED)
        );
        assert_eq!(engine.state().players[0].mana_pool.total(), 0);
        bury(&mut engine, &[vane]);
        pass_until(&mut engine, stack_is_empty);
        reach_their_main_phase(&mut engine, P1);
        let after = engine.state().object(target).unwrap().characteristics();
        assert_eq!(
            after.supertypes,
            before.supertypes.difference(SupertypeSet::SNOW)
        );
        assert_eq!(after.types, before.types);
        assert_eq!(after.subtypes, before.subtypes);
    }
}

#[test]
fn weathervane_adds_snow_only_to_nonsnow_basic_lands_and_keeps_other_types() {
    let mut engine = prepared();
    let vane = on_battlefield(&engine, P0, arcum_s_weathervane()).unwrap();
    let target = on_battlefield(&engine, P1, forest()).unwrap();
    let before = engine
        .state()
        .object(target)
        .unwrap()
        .characteristics()
        .clone();
    pay_two(&mut engine);
    activate(&mut engine, P0, arcum_s_weathervane(), 1);
    let options = aim_at(&mut engine, P0, target);
    assert_eq!(
        options.len(),
        3,
        "two own Forests and the opponent's nonsnow Forest"
    );
    assert!(options.contains(&target));
    for excluded in [
        index::SNOW_COVERED_FOREST,
        index::SNOWFIELD_SINKHOLE,
        index::SOL_RING,
    ] {
        assert!(!options.contains(&on_battlefield(&engine, P1, excluded).unwrap()));
    }
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    bury(&mut engine, &[vane]);
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, P1);
    let after = engine.state().object(target).unwrap().characteristics();
    assert_eq!(
        after.supertypes,
        before.supertypes.union(SupertypeSet::SNOW)
    );
    assert_eq!(after.types, before.types);
    assert_eq!(after.subtypes, before.subtypes);
}

#[test]
fn weathervane_does_not_change_a_land_that_left_and_returned_before_resolution() {
    let mut engine = prepared();
    let target = on_battlefield(&engine, P1, forest()).unwrap();
    pay_two(&mut engine);
    activate(&mut engine, P0, arcum_s_weathervane(), 1);
    aim_at(&mut engine, P0, target);
    let state = engine.dev_state_mut(P0).unwrap();
    state
        .move_object(
            target,
            ZoneLocation::Exile(P1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            target,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !engine
            .state()
            .object(target)
            .unwrap()
            .characteristics()
            .supertypes
            .contains(SupertypeSet::SNOW)
    );
}
