//! `cards/lands/utility/ominous_cemetery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ominous Cemetery: "{T}: Add {C}." / "{5}, {T}, Exile this land: Target creature's owner shuffles it into their library."
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn ominous_cemetery_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(120, forest())
        .battlefield(0, &[ominous_cemetery()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cemetery = on_battlefield(&engine, p0, ominous_cemetery()).expect("Cemetery deployed");
    activate(&mut engine, p0, ominous_cemetery(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, cemetery));
}

#[test]
fn cemetery_shuffles_into_the_owners_library_and_exiles_itself_as_cost() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[ominous_cemetery(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let cemetery = on_battlefield(&e, p0, ominous_cemetery()).unwrap();
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let other_library = e.state().zones.list(ZoneLocation::Library(p1)).clone();
    let state = e.dev_state_mut(p0).unwrap();
    state.object_mut(bear).unwrap().set_controller(p1);
    state.invalidate_projections();
    state.players[0].mana_pool.add(ManaColor::Colorless, 5);
    e.refresh_offer();
    activate(&mut e, p0, ominous_cemetery(), 1);
    aim_at(&mut e, p0, bear);
    assert_eq!(e.state().object(cemetery).unwrap().zone, Zone::Exile);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .zones
            .list(ZoneLocation::Library(p0))
            .contains(&bear)
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(p1)),
        &other_library
    );
}

#[test]
fn cemetery_asks_before_shuffling_a_commander_into_its_library() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for home in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[ominous_cemetery(), plains()])
            .commander(0, &[index::SAVANNAH_LIONS])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let commander = e.state().commanders[0][0].object;
        tap_all_mana(&mut e, p0);
        e.apply(p0, PlayerAction::CastSpell { card: commander })
            .unwrap();
        pass_until(&mut e, stack_is_empty);
        let cemetery = on_battlefield(&e, p0, ominous_cemetery()).unwrap();
        let state = e.dev_state_mut(p0).unwrap();
        state.set_tapped(cemetery, false);
        state.players[0].mana_pool.add(ManaColor::Colorless, 5);
        e.refresh_offer();
        activate(&mut e, p0, ominous_cemetery(), 1);
        aim_at(&mut e, p0, commander);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        assert_eq!(e.state().object(commander).unwrap().zone, Zone::Battlefield);
        e.apply(p0, PlayerAction::YesNo(home)).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(commander).unwrap().zone,
            if home { Zone::Command } else { Zone::Library }
        );
    }
}

/// "Target creature's owner shuffles it into their library.": only creatures
/// are on the menu, not the artifact or the land on the same side.
#[test]
fn cemetery_offers_creatures_only() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                ominous_cemetery(),
                index::SOL_RING,
                index::FOREST,
                index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 5);
    e.refresh_offer();
    activate(&mut e, p0, ominous_cemetery(), 1);
    let options = aim_at(&mut e, p0, bear);
    assert_eq!(
        options,
        vec![bear],
        "a creature's owner shuffles it: no Sol Ring, no Forest"
    );
}
