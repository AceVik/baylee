//! `cards/instants/mv_6/tsabo_s_decree.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn tsabo_s_decree() -> CardIndex {
    card_index("4e0de4d9-7a30-4a2b-94ba-8239baad2a0a")
}

fn drudge_skeletons() -> CardIndex {
    card_index("c180ed02-07f4-4538-9bea-8c249234a8e2")
}

fn woodland_changeling() -> CardIndex {
    card_index("338f0ab6-f744-4e5a-a358-05e71b93758d")
}

/// Tsabo's Decree makes target player discard all creature cards of the chosen
/// type (including changeling creatures, sparing noncreature kindred spells and
/// wrong-type creatures) and destroys all creatures of that type that player
/// controls without allowing regeneration.
#[test]
fn tsabo_s_decree_discards_and_destroys_without_regeneration() {
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
                drudge_skeletons(),
            ],
        )
        .battlefield(
            1,
            &[
                swamp(),
                drudge_skeletons(),
                woodland_changeling(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[tsabo_s_decree()])
        .hand(
            1,
            &[
                drudge_skeletons(),
                woodland_changeling(),
                festering_goblin(),
                crib_swap(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_skeleton = on_battlefield(&engine, p0, drudge_skeletons()).expect("P0 Skeleton");
    let opp_skeleton = on_battlefield(&engine, p1, drudge_skeletons()).expect("P1 Skeleton");
    let opp_changeling = on_battlefield(&engine, p1, woodland_changeling()).expect("P1 Changeling");
    let opp_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("P1 Elf");

    cast_from_hand(&mut engine, p0, tsabo_s_decree());
    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("target opponent with Tsabo's Decree");
    // Pass P0's priority so P1 can set up a regeneration shield on Drudge Skeletons.
    pass_once(&mut engine);
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, drudge_skeletons(), 0);
    // Let the regeneration ability resolve cleanly on top of Tsabo's Decree.
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

    // Hand: Skeleton & Changeling creatures were discarded; Goblin creature & noncreature Crib Swap stayed.
    assert!(in_hand(&engine, p1, drudge_skeletons()).is_none());
    assert!(in_hand(&engine, p1, woodland_changeling()).is_none());
    assert!(in_hand(&engine, p1, festering_goblin()).is_some());
    assert!(
        in_hand(&engine, p1, crib_swap()).is_some(),
        "Crib Swap is not a creature card"
    );

    // Battlefield: P1's regenerated Drudge Skeletons and Changeling are destroyed;
    // P1's non-Skeleton Elf and P0's Drudge Skeletons survive.
    assert_eq!(zone_of(&engine, opp_skeleton), Zone::Graveyard);
    assert_eq!(zone_of(&engine, opp_changeling), Zone::Graveyard);
    assert_eq!(zone_of(&engine, opp_elf), Zone::Battlefield);
    assert_eq!(zone_of(&engine, my_skeleton), Zone::Battlefield);
}
