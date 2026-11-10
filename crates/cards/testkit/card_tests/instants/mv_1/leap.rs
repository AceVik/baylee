//! `cards/instants/mv_1/leap.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Leap is `{U}` for two printed lines — "Target creature gains flying until
/// end of turn" and "Draw a card" — and both are only themselves together: a
/// card that granted flying without drawing is a different card, and one that
/// drew without granting reads the same on a library count. The board carries
/// two Elves under one seat and a third across the table, so the keyword has
/// to land on the creature the target question *named* and on no other —
/// "target creature" is neither "creatures you control" nor the whole table —
/// and the draw is read as a library one card shorter, which a reveal or a
/// scry could not produce.
#[test]
fn leap_grants_flying_to_the_creature_it_names_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[leap()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays grounded");
    let (chosen, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, chosen).contains(KeywordSet::FLYING),
        "nothing has been cast yet"
    );

    // The Island is the whole price, and the Elves are named as the printing
    // kept back: a creature tapped for mana is a creature whose status has
    // already changed for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Island and neither Elf: the {{U}} is a real payment"
    );

    let library_before = library_size(&engine, p0);
    cast_with_floating(&mut engine, p0, leap());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast the spell aims it");
    assert!(
        options.contains(&chosen) && options.contains(&bystander),
        "both creatures you control may be the target: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("the creature the question offered was chosen");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, chosen).contains(KeywordSet::FLYING),
        "\"target creature gains flying until end of turn\""
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING),
        "the spell reaches the creature it named and no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "nor across the table, where nobody was named"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" — one card left the top of the library"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{U}} came out of the pool"
    );
}
