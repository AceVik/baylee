//! `cards/instants/mv_2/standardize.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

fn standardize() -> CardIndex {
    card_index("2d4e0e0f-7167-40a4-ad76-2d5b45223eaa")
}

/// Standardize excludes Wall from choices, turns every creature currently on
/// the battlefield into the chosen type until end of turn, leaves creatures
/// entering later untouched (CR 611.2c), and reverts at cleanup.
#[test]
fn standardize_changes_all_current_creatures_except_wall_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                forest(),
                llanowar_elves(),
                serra_angel(),
            ],
        )
        .battlefield(1, &[festering_goblin(), wall_of_swords()])
        .hand(0, &[standardize(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf0 = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 Elf");
    let angel0 = on_battlefield(&engine, p0, serra_angel()).expect("P0 Angel");
    let goblin1 = on_battlefield(&engine, p1, festering_goblin()).expect("P1 Goblin");
    let wall1 = on_battlefield(&engine, p1, wall_of_swords()).expect("P1 Wall");

    let blue: Vec<ObjectId> = all_on_battlefield(&engine, p0, island());
    tap_mana_where(&mut engine, p0, |id| blue.contains(&id));
    cast_with_floating(&mut engine, p0, standardize());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        unreachable!("matched ChooseSubtype");
    };
    assert_eq!(player, p0);
    assert!(
        !options.contains(&subtypes::creature::WALL),
        "Wall is excluded from Standardize choices"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::WALL))
            .is_err()
    );
    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::BIRD))
        .expect("choose Bird");
    pass_until(&mut engine, |e| at_rest(e, p0));

    for id in [elf0, angel0, goblin1, wall1] {
        let subs = &engine
            .state()
            .object(id)
            .unwrap()
            .characteristics()
            .subtypes;
        assert!(subs.contains(subtypes::creature::BIRD));
        assert!(!subs.contains(subtypes::creature::ELF));
        assert!(!subs.contains(subtypes::creature::GOBLIN));
        assert!(!subs.contains(subtypes::creature::WALL));
    }

    // CR 611.2c: a creature entering after Standardize resolves keeps its printed subtypes.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let new_elf = all_on_battlefield(&engine, p0, llanowar_elves())
        .into_iter()
        .find(|id| *id != elf0)
        .expect("second Elf entered");
    let new_subs = &engine
        .state()
        .object(new_elf)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(new_subs.contains(subtypes::creature::ELF));
    assert!(!new_subs.contains(subtypes::creature::BIRD));

    // Reverts at cleanup.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        engine
            .state()
            .object(angel0)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::ANGEL)
    );
    assert!(
        engine
            .state()
            .object(wall1)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::creature::WALL)
    );
}
