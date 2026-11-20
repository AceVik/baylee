//! `cards/creatures/artifacts/mv_6/brass_herald.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn brass_herald() -> CardIndex {
    card_index("ea9b2e3d-2de4-4cde-b233-6d0ba5797c20")
}

fn woodland_changeling() -> CardIndex {
    card_index("338f0ab6-f744-4e5a-a358-05e71b93758d")
}

/// Brass Herald names a creature type as it enters, puts every creature card of
/// that type from the top four cards into hand and orders the rest onto the
/// bottom, and gives creatures of that type +1/+1 across both players without
/// gaining that type or pumping itself.
#[test]
fn brass_herald_reveals_top_four_takes_matching_creatures_and_pumps_type() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(
            0,
            &[
                brass_herald(),
                llanowar_elves(),
                woodland_changeling(),
                festering_goblin(),
                crib_swap(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Seed top 4 of library: 2 matching creature cards (Elf + Changeling
    // creature) and 2 non-matches (wrong-type Goblin + noncreature Crib Swap).
    let miss_noncreature = hand_to_library_top(&mut engine, p0, crib_swap());
    let miss_goblin = hand_to_library_top(&mut engine, p0, festering_goblin());
    hand_to_library_top(&mut engine, p0, woodland_changeling());
    hand_to_library_top(&mut engine, p0, llanowar_elves());
    let lib_before = library_size(&engine, p0);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf seated");
    let my_goblin = on_battlefield(&engine, p0, festering_goblin()).expect("P0 Goblin seated");
    let opp_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf seated");

    cast_from_hand(&mut engine, p0, brass_herald());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ELF))
        .expect("choose Elf for Brass Herald");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { player, cards, .. } = engine.pending().clone() else {
        unreachable!("matched bottom-order arrange");
    };
    assert_eq!(player, p0);
    assert_eq!(cards.len(), 2);
    assert!(cards.contains(&miss_goblin));
    assert!(cards.contains(&miss_noncreature));
    engine
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![vec![miss_noncreature, miss_goblin]],
            },
        )
        .expect("order the two non-matching cards onto the bottom");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_some(),
        "Elf creature card moved to hand"
    );
    assert!(
        in_hand(&engine, p0, woodland_changeling()).is_some(),
        "Changeling creature card counts as Elf and moved to hand"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_before - 2,
        "2 taken to hand, 2 placed back on bottom"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0))[..2],
        [miss_goblin, miss_noncreature],
        "misses sit at the bottom in chosen order (pile[0] shallowest)"
    );

    let herald = on_battlefield(&engine, p0, brass_herald()).expect("Brass Herald seated");
    let herald_subtypes = &engine
        .state()
        .object(herald)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(herald_subtypes.contains(subtypes::creature::GOLEM));
    assert!(
        !herald_subtypes.contains(subtypes::creature::ELF),
        "Brass Herald stays Golem only"
    );
    assert_eq!(
        pt(&engine, herald),
        (2, 2),
        "Brass Herald does not pump itself"
    );
    assert_eq!(pt(&engine, my_elf), (2, 2), "controller's Elf gets +1/+1");
    assert_eq!(
        pt(&engine, opp_elf),
        (2, 2),
        "opponent's Elf also gets +1/+1"
    );
    assert_eq!(
        pt(&engine, my_goblin),
        (1, 1),
        "non-Elf creature is unpumped"
    );
}
