//! `cards/creatures/mv_5/riptide_shapeshifter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn riptide_shapeshifter() -> CardIndex {
    card_index("49f292ab-0533-436b-a309-8bc795b66e5c")
}

/// Riptide Shapeshifter sacrifices itself for `{2}{U}{U}`, reveals cards from
/// the top until finding a creature card of the chosen type (skipping wrong
/// types and noncreature changeling cards), puts that creature onto the
/// battlefield, and shuffles all other revealed cards back into the library.
#[test]
fn riptide_shapeshifter_puts_first_matching_creature_and_shuffles_rest() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                riptide_shapeshifter(),
            ],
        )
        .hand(0, &[serra_angel(), crib_swap(), festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shifter = on_battlefield(&engine, p0, riptide_shapeshifter()).expect("Shapeshifter seated");
    let angel_in_lib = hand_to_library_top(&mut engine, p0, serra_angel());
    let spell_miss = hand_to_library_top(&mut engine, p0, crib_swap());
    let goblin_miss = hand_to_library_top(&mut engine, p0, festering_goblin());
    let lib_before = library_size(&engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, riptide_shapeshifter(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    assert_eq!(
        zone_of(&engine, shifter),
        Zone::Graveyard,
        "sacrificed as activation cost"
    );
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ANGEL))
        .expect("choose Angel");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(zone_of(&engine, angel_in_lib), Zone::Battlefield);
    assert_eq!(
        zone_of(&engine, spell_miss),
        Zone::Library,
        "noncreature Changeling skipped and shuffled back"
    );
    assert_eq!(
        zone_of(&engine, goblin_miss),
        Zone::Library,
        "wrong-type Goblin skipped and shuffled back"
    );
    assert_eq!(library_size(&engine, p0), lib_before - 1);
}

/// Naming a creature type absent from the library shuffles cleanly with nothing deployed.
#[test]
fn riptide_shapeshifter_zero_matches_shuffles_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                riptide_shapeshifter(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shifter = on_battlefield(&engine, p0, riptide_shapeshifter()).expect("Shapeshifter seated");
    let lib_before = library_size(&engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, riptide_shapeshifter(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::DRAGON))
        .expect("choose Dragon with no Dragons in library");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(zone_of(&engine, shifter), Zone::Graveyard);
    assert_eq!(library_size(&engine, p0), lib_before);
}
