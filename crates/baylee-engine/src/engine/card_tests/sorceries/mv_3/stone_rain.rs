//! `cards/sorceries/mv_3/stone_rain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stone Rain — {2}{R} sorcery: "Destroy target land."
///
/// "Target land" names no side of the table, so lands stand on both sides and
/// a permanent that is not one stands beside them: the Elf is a permanent this
/// seat controls, and a filter that had lost the type would offer it exactly
/// as readily. The two lands across the table are the other half — one is
/// named and dies, the other is not and stands — which is what tells a
/// targeted destroy from a board sweep. The mana is read both while the target
/// question is still open and once it has been answered, because CR 601.2c
/// names the target before CR 601.2h pays for it.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn stone_rain_destroys_the_one_land_it_names_and_leaves_every_other_land_standing() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .battlefield(1, &[forest(), island()])
        .hand(0, &[stone_rain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let their_island = on_battlefield(&engine, p1, island()).expect("their Island is out");
    let mine = all_on_battlefield(&engine, p0, mountain());
    assert_eq!(
        mine.len(),
        3,
        "three Mountains, which is exactly {{2}}{{R}}"
    );

    // Every mana source is tapped before anything is claimed about the cast,
    // because `can_afford` reads the pool and not the untapped lands. The Elf
    // is one of them: its whole price is its own {{T}}, so `tap_all_mana`
    // presses it and its green counts (CR 605.1).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "three Mountains and the Elf: the red alone pays {{2}}{{R}}"
    );
    cast_with_floating(&mut engine, p0, stone_rain());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"destroy target land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one land, and the spell asks once");
    assert!(
        options.contains(&their_forest) && options.contains(&their_island),
        "\"target land\" reaches across the table: {options:?}"
    );
    assert!(
        mine.iter().all(|land| options.contains(land)),
        "and it reaches this seat's own lands, tapped or not: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the Elf is a permanent this seat controls and no land: {options:?}"
    );
    assert_eq!(
        options.len(),
        5,
        "three Mountains and the two lands across the table, and nothing else: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "CR 601.2h pays last, so the cost is still in the pool while the target \
         is unanswered"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and the land the spell named has not moved yet"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_forest],
            },
        )
        .expect("the Forest was one of the options the spell enumerated");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}}{{R}} came out of the pool: four mana were floating and one is left"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "\"destroy target land\": the named Forest is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "and it left the battlefield, which is the destruction itself"
    );
    assert!(
        on_battlefield(&engine, p1, island()).is_some(),
        "the land the spell did not name never moved"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, mountain()).len(),
        3,
        "and neither did the three Mountains it could have named instead"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "nor the permanent the filter declined"
    );
}
