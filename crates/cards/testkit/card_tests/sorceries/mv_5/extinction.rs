//! `cards/sorceries/mv_5/extinction.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn extinction() -> CardIndex {
    card_index("f3f01fe8-0a2c-4afa-94d2-268d27c0d252")
}

fn drudge_skeletons() -> CardIndex {
    card_index("c180ed02-07f4-4538-9bea-8c249234a8e2")
}

fn woodland_changeling() -> CardIndex {
    card_index("338f0ab6-f744-4e5a-a358-05e71b93758d")
}

/// Extinction destroys all creatures of the chosen type across every player's
/// battlefield (including changelings, while sparing other creature types), and
/// allows normal regeneration.
#[test]
fn extinction_destroys_all_creatures_of_chosen_type_with_regeneration_allowed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
                festering_goblin(),
                woodland_changeling(),
            ],
        )
        .battlefield(
            1,
            &[swamp(), llanowar_elves(), drudge_skeletons(), serra_angel()],
        )
        .hand(0, &[extinction()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf0 = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf");
    let goblin0 = on_battlefield(&engine, p0, festering_goblin()).expect("P0 Goblin");
    let changeling0 = on_battlefield(&engine, p0, woodland_changeling()).expect("P0 Changeling");
    let elf1 = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf");
    let skeleton1 = on_battlefield(&engine, p1, drudge_skeletons()).expect("P1 Skeleton");
    let angel1 = on_battlefield(&engine, p1, serra_angel()).expect("P1 Angel");

    cast_from_hand(&mut engine, p0, extinction());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf for Extinction");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(zone_of(&engine, elf0), Zone::Graveyard);
    assert_eq!(zone_of(&engine, changeling0), Zone::Graveyard);
    assert_eq!(zone_of(&engine, elf1), Zone::Graveyard);
    assert_eq!(zone_of(&engine, goblin0), Zone::Battlefield);
    assert_eq!(zone_of(&engine, skeleton1), Zone::Battlefield);
    assert_eq!(zone_of(&engine, angel1), Zone::Battlefield);
}

/// Unlike Tsabo's Decree, Extinction's destruction permits regeneration.
#[test]
fn extinction_allows_regeneration() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[swamp(), drudge_skeletons()])
        .hand(0, &[extinction()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let skeleton1 = on_battlefield(&engine, p1, drudge_skeletons()).expect("P1 Skeleton");
    cast_from_hand(&mut engine, p0, extinction());
    pass_once(&mut engine);
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, drudge_skeletons(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseSubtype(subtypes::creature::SKELETON),
        )
        .expect("choose Skeleton");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        zone_of(&engine, skeleton1),
        Zone::Battlefield,
        "regeneration shield saves Drudge Skeletons from Extinction"
    );
    assert!(is_tapped(&engine, skeleton1));
}
