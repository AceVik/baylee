//! `cards/lands/planar_nexus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn planar_nexus_filters_mana_into_any_chosen_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[planar_nexus(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana_but(&mut engine, p0, Some(planar_nexus()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );

    activate(&mut engine, p0, planar_nexus(), 1);

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(options.contains(&ManaColor::Blue));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.available(ManaColor::Green), 0);
    assert_eq!(pool.total(), 1);

    let nexus = on_battlefield(&engine, p0, planar_nexus()).expect("nexus on battlefield");
    assert!(is_tapped(&engine, nexus));
}

#[test]
fn planar_nexus_has_every_nonbasic_land_type_in_every_zone_and_completes_tron() {
    use baylee_core::{
        generated::{index, subtypes},
        types::SubtypeSet,
    };
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::PLANAR_NEXUS, index::URZA_S_TOWER])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let nexus = on_battlefield(&e, p0, index::PLANAR_NEXUS).unwrap();
    let expected = subtypes::ALL_LAND_TYPES.difference(SubtypeSet::BASIC_LANDS);
    assert_eq!(
        e.state().object(nexus).unwrap().characteristics().subtypes,
        expected
    );
    activate(&mut e, p0, index::URZA_S_TOWER, 0);
    assert_eq!(
        e.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        3
    );
    for zone in [
        ZoneLocation::Hand(p0),
        ZoneLocation::Library(p0),
        ZoneLocation::Graveyard(p0),
        ZoneLocation::Exile(p0),
    ] {
        e.dev_state_mut(p0)
            .unwrap()
            .move_object(
                nexus,
                zone,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        assert_eq!(
            e.state().object(nexus).unwrap().characteristics().subtypes,
            expected
        );
    }
}

#[test]
fn planar_nexus_loses_its_land_types_under_evil_presence_and_regains_them_when_it_leaves() {
    use baylee_core::{
        generated::{index, subtypes},
        types::SubtypeSet,
    };
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::PLANAR_NEXUS])
        .hand(0, &[index::EVIL_PRESENCE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let nexus = on_battlefield(&e, p0, index::PLANAR_NEXUS).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Black, 1);
    e.refresh_offer();
    cast_from_hand(&mut e, p0, index::EVIL_PRESENCE);
    aim_at(&mut e, p0, nexus);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state().object(nexus).unwrap().characteristics().subtypes,
        SubtypeSet::from_slice(&[subtypes::land::SWAMP])
    );
    let moon = on_battlefield(&e, p0, index::EVIL_PRESENCE).unwrap();
    bury(&mut e, &[moon]);
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        e.state().object(nexus).unwrap().characteristics().subtypes,
        subtypes::ALL_LAND_TYPES.difference(SubtypeSet::BASIC_LANDS)
    );
}
