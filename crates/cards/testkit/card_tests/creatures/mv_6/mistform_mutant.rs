//! `cards/creatures/mv_6/mistform_mutant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn mistform_mutant() -> CardIndex {
    card_index("0f4350e2-48c7-4d2b-9384-4864232756ad")
}

/// Mistform Mutant's `{1}{U}` ability excludes Wall from choices, turns target
/// creature into the chosen type until end of turn, and reverts at cleanup.
#[test]
fn mistform_mutant_changes_target_type_excluding_wall_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), mistform_mutant()])
        .battlefield(1, &[wall_of_swords()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p1, wall_of_swords()).expect("Wall seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, mistform_mutant(), 0);
    aim(&mut engine, &[wall], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        unreachable!("matched ChooseSubtype");
    };
    assert_eq!(player, p0);
    assert!(
        !options.contains(&subtypes::creature::WALL),
        "Wall is excluded from Mistform Mutant"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::WALL))
            .is_err()
    );
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::ANGEL))
        .expect("choose Angel");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let subs = &engine
        .state()
        .object(wall)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs.contains(subtypes::creature::ANGEL));
    assert!(!subs.contains(subtypes::creature::WALL));

    reach_their_main_phase(&mut engine, p1);
    assert!(
        engine
            .state()
            .object(wall)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::WALL)
    );
}
