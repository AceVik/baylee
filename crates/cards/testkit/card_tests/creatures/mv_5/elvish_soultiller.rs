//! `cards/creatures/mv_5/elvish_soultiller.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn elvish_soultiller() -> CardIndex {
    card_index("10b0b60f-9976-41f9-855b-1786b6f86f55")
}

fn woodland_changeling() -> CardIndex {
    card_index("338f0ab6-f744-4e5a-a358-05e71b93758d")
}

/// When Elvish Soultiller dies, you choose a creature type and shuffle every
/// creature card of that type (including Elvish Soultiller itself and
/// changeling creatures, while leaving wrong-type creatures and noncreature
/// changeling spells behind) from your graveyard into your library.
#[test]
fn elvish_soultiller_shuffles_matching_graveyard_creatures_on_death() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), elvish_soultiller()])
        .hand(
            0,
            &[
                lightning_bolt(),
                lightning_bolt(),
                llanowar_elves(),
                woodland_changeling(),
                festering_goblin(),
                crib_swap(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let soultiller = on_battlefield(&engine, p0, elvish_soultiller()).expect("Soultiller seated");
    let gy_elf = hand_to_graveyard(&mut engine, p0, llanowar_elves());
    let gy_changeling = hand_to_graveyard(&mut engine, p0, woodland_changeling());
    let gy_goblin = hand_to_graveyard(&mut engine, p0, festering_goblin());
    let gy_noncreature = hand_to_graveyard(&mut engine, p0, crib_swap());
    let lib_before = library_size(&engine, p0);

    // Deal 6 damage across two Lightning Bolts to destroy 5/4 Elvish Soultiller.
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim(&mut engine, &[soultiller], &[]);
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim(&mut engine, &[soultiller], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(zone_of(&engine, soultiller), Zone::Library);
    assert_eq!(zone_of(&engine, gy_elf), Zone::Library);
    assert_eq!(zone_of(&engine, gy_changeling), Zone::Library);
    assert_eq!(
        zone_of(&engine, gy_goblin),
        Zone::Graveyard,
        "wrong-type Goblin remains in graveyard"
    );
    assert_eq!(
        zone_of(&engine, gy_noncreature),
        Zone::Graveyard,
        "noncreature Kindred Changeling remains in graveyard"
    );
    assert_eq!(library_size(&engine, p0), lib_before + 3);
}

/// Naming a creature type absent from the graveyard leaves Elvish Soultiller in
/// the graveyard and settles cleanly.
#[test]
fn elvish_soultiller_zero_matches_leaves_all_in_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), elvish_soultiller()])
        .hand(0, &[lightning_bolt(), lightning_bolt(), festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let soultiller = on_battlefield(&engine, p0, elvish_soultiller()).expect("Soultiller seated");
    let gy_goblin = hand_to_graveyard(&mut engine, p0, festering_goblin());
    let lib_before = library_size(&engine, p0);

    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim(&mut engine, &[soultiller], &[]);
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim(&mut engine, &[soultiller], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::DRAGON))
        .expect("choose Dragon with none in graveyard");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(zone_of(&engine, soultiller), Zone::Graveyard);
    assert_eq!(zone_of(&engine, gy_goblin), Zone::Graveyard);
    assert_eq!(library_size(&engine, p0), lib_before);
}
