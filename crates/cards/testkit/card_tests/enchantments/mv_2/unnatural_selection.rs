//! `cards/enchantments/mv_2/unnatural_selection.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;
use baylee_core::generated::subtypes;

fn unnatural_selection() -> CardIndex {
    card_index("0be10525-09dc-4970-b495-2db7d4d3c3e7")
}

/// Unnatural Selection's `{1}` ability excludes Wall from the choice menu,
/// replaces target creature's creature type with the chosen type until end of
/// turn, and wears off at cleanup.
#[test]
fn unnatural_selection_changes_type_excluding_wall_until_eot() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .hand(0, &[unnatural_selection()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf seated");
    cast_from_hand(&mut engine, p0, unnatural_selection());
    pass_until(&mut engine, stack_is_empty);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, unnatural_selection(), 0);
    aim(&mut engine, &[elf], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        unreachable!("matched ChooseSubtype");
    };
    assert_eq!(player, p0);
    assert!(
        !options.contains(&subtypes::creature::WALL),
        "Wall cannot be chosen"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::WALL))
            .is_err(),
        "choosing Wall explicitly must fail validation"
    );

    engine
        .apply(p0, PlayerAction::ChooseSubtype(subtypes::creature::GOBLIN))
        .expect("choose Goblin");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let subs = &engine
        .state()
        .object(elf)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(subs.contains(subtypes::creature::GOBLIN));
    assert!(!subs.contains(subtypes::creature::ELF));
    assert!(!subs.contains(subtypes::creature::DRUID));

    // Wears off after cleanup at end of turn.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    let subs_after = &engine
        .state()
        .object(elf)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(!subs_after.contains(subtypes::creature::GOBLIN));
    assert!(subs_after.contains(subtypes::creature::ELF));
}
