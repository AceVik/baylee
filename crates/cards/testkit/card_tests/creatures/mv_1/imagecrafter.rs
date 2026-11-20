//! `cards/creatures/mv_1/imagecrafter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn imagecrafter() -> CardIndex {
    card_index("9a0c6b4d-b16a-4849-9106-047b009a99b3")
}

/// Imagecrafter's `{T}` ability excludes Wall from choices, turns target
/// creature into the chosen type until end of turn, and reverts at cleanup.
#[test]
fn imagecrafter_taps_to_change_target_type_excluding_wall_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[imagecrafter()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let crafter = on_battlefield(&engine, p0, imagecrafter()).expect("Imagecrafter seated");
    let opp_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent Elf seated");

    activate(&mut engine, p0, imagecrafter(), 0);
    aim(&mut engine, &[opp_elf], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        unreachable!("matched ChooseSubtype");
    };
    assert_eq!(player, p0);
    assert!(
        !options.contains(&subtypes::creature::WALL),
        "Wall is excluded from Imagecrafter choices"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::WALL))
            .is_err()
    );
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::GOBLIN))
        .expect("choose Goblin");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(is_tapped(&engine, crafter));
    let subs = &engine
        .state()
        .object(opp_elf)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs.contains(subtypes::creature::GOBLIN));
    assert!(!subs.contains(subtypes::creature::ELF));

    reach_their_main_phase(&mut engine, p1);
    let subs_after = &engine
        .state()
        .object(opp_elf)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs_after.contains(subtypes::creature::ELF));
    assert!(!subs_after.contains(subtypes::creature::GOBLIN));
}
