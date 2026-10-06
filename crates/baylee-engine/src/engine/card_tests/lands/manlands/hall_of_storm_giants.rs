//! `cards/lands/manlands/hall_of_storm_giants.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Forgotten Realms creature-lands and Thran Portal, played from hand
/// on both sides of their own bound.
///
/// All four joined `LANDS_THAT_WOULD_COUNT_THEMSELVES` when they were
/// written, because their filter matches the land that is arriving — so the
/// only thing keeping them from turning off one land early is
/// `controls_at_least` skipping the entering permanent, and the list says a
/// new arrival owes a played test.
///
/// Both sides of every bound, because a test of one side passes against a
/// clause that is simply always true or always false. And the bound is not
/// the same for all four: the manlands print "if you control two or more
/// **other** lands, this land enters tapped" (`at_most: 1`) and Thran
/// Portal prints "unless you control two or fewer other lands"
/// (`at_most: 2`), which is the same predicate from opposite ends.
///
/// Three of these four also carried the defect that
/// `every_entry_clause_is_scoped_to_its_controller` now lints: a
/// bare `Filter::LAND` counts the lands **across the table**, because
/// `controls_count` scopes nothing. So the untapped half of each pair is
/// set up with lands on the opponent's side of the table as well, which is
/// the board that told the two apart.
#[test]
fn a_counted_entry_clause_counts_only_its_controller_s_other_lands() {
    let p0 = PlayerId::new(0);
    // (the land, how many of my lands still let it enter untapped)
    let cases: [(CardIndex, usize); 4] = [
        (hall_of_storm_giants(), 1),
        (den_of_the_bugbear(), 1),
        (hive_of_the_eye_tyrant(), 1),
        (thran_portal(), 2),
    ];

    for (land, at_most) in cases {
        // At the bound, and with a full opposing board: untapped, and
        // counting the other seat's lands would tap it.
        let mine = vec![island(); at_most];
        let mut engine = Duel::new(217, forest())
            .battlefield(0, &mine)
            .battlefield(1, &[island(), island(), island(), island()])
            .hand(0, &[land])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let card = in_hand(&engine, p0, land).expect("the land is in hand");
        engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert!(
            !entered_tapped(&engine, card),
            "at the bound with four lands across the table, it enters untapped"
        );

        // One over the bound, on my own side: tapped.
        let mine = vec![island(); at_most + 1];
        let mut engine = Duel::new(218, forest())
            .battlefield(0, &mine)
            .hand(0, &[land])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let card = in_hand(&engine, p0, land).expect("the land is in hand");
        engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert!(
            entered_tapped(&engine, card),
            "one land over the bound, it enters tapped"
        );
    }
}

/// Hall of Storm Giants: "If you control two or more other lands, this land enters tapped." / "{T}: Add {U}." / "{5}{U}: Until end of turn, this land becomes a 7/7 blue Giant creature with ward {3}. It's still a land."
/// The granted ward `{3}` and stacked animations are exercised in `ward_tests`.
/// After untapping, paying `{5}{U}` animates the land into a 7/7 blue Giant creature that remains a land.
#[test]
fn hall_of_storm_giants_animates_into_giant() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(216, forest())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[hall_of_storm_giants()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall = play_land(&mut engine, p0, hall_of_storm_giants());
    assert!(entered_tapped(&engine, hall));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, hall));

    tap_mana_except(&mut engine, p0, hall);
    activate(&mut engine, p0, hall_of_storm_giants(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, hall), (7, 7));
    let types = engine.state().object(hall).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, hall));
}
