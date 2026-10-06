//! `cards/enchantments/mv_3/garruk_s_uprising.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Garruk's Uprising` (`Coverage::Partial`):
/// "When this enchantment enters, if you control a creature with power 4 or greater, draw a card.
/// Creatures you control have trample. Whenever a creature you control with power 4 or greater enters,
/// draw a card."
///
/// This is the third line, read about a creature *entering*: the trample
/// anthem reaches your creatures and not the opponent's, casting a 1/1
/// draws nothing, and casting a 6/6 draws. The enchantment is seated rather
/// than cast here, so its own enters-trigger never fires — that one is the
/// test above, and keeping them apart is what stops one draw being read as
/// the other.
#[test]
fn garruk_s_uprising_grants_trample_and_draws_on_power_four_or_greater() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(105, forest())
        .battlefield(
            0,
            &[
                garruk_s_uprising(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[llanowar_elves(), rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let their_elf =
        on_battlefield(&engine, p1, llanowar_elves()).expect("opponent's elf on battlefield");
    assert!(
        !keywords(&engine, their_elf).contains(KeywordSet::TRAMPLE),
        "Garruk's Uprising only grants trample to creatures you control"
    );

    let lib_start = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf on battlefield");
    assert!(
        keywords(&engine, my_elf).contains(KeywordSet::TRAMPLE),
        "controlled creature has trample from the anthem"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_start,
        "a 1/1 entering does not trigger the draw trigger"
    );

    cast_from_hand(&mut engine, p0, rootbreaker_wurm());
    pass_until(&mut engine, stack_is_empty);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("wurm on battlefield");
    assert!(
        keywords(&engine, wurm).contains(KeywordSet::TRAMPLE),
        "wurm has trample"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_start - 1,
        "a 6/6 creature entering triggers the draw ability"
    );
}

/// Garruk's Uprising prints three lines and the **first** one is an
/// intervening `if` on the enchantment's own arrival: "When this enchantment
/// enters, if you control a creature with power 4 or greater, draw a card."
/// CR 603.4 checks it twice, and both checks are against the board as the
/// enchantment lands — so the 6/6 already standing earns the card and a 1/1
/// standing in its place earns nothing. The test beside this one plays the
/// third line, which is the same predicate read about a creature entering
/// rather than about the board.
#[test]
fn garruk_s_uprising_draws_on_its_own_arrival_only_over_a_four_power_creature() {
    let p0 = PlayerId::new(0);

    let draw_from = |creature: CardIndex, seed: u64| -> usize {
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &[forest(), forest(), forest(), forest(), creature])
            .hand(0, &[garruk_s_uprising()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let before = library_size(&engine, p0);
        cast_from_hand(&mut engine, p0, garruk_s_uprising());
        pass_until(&mut engine, stack_is_empty);
        assert!(
            on_battlefield(&engine, p0, garruk_s_uprising()).is_some(),
            "the enchantment resolved either way"
        );
        before - library_size(&engine, p0)
    };

    assert_eq!(
        draw_from(llanowar_elves(), 9301),
        0,
        "a 1/1 is not \"a creature with power 4 or greater\", so the trigger \
         is binned by its own intervening if"
    );
    assert_eq!(
        draw_from(rootbreaker_wurm(), 9302),
        1,
        "a 6/6 earns the card as the enchantment lands"
    );
}
