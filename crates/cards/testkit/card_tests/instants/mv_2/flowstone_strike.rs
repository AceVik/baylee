//! `cards/instants/mv_2/flowstone_strike.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flowstone Strike prints one line — "Target creature gets +1/-1 and gains
/// haste until end of turn" — and one casting reads all of it. `(7, 5)` on a
/// printed 6/6 is the only body that applies a power *up* and a toughness
/// *down*: a symmetric pump would leave `(7, 7)` and a swapped pair would read
/// `(5, 7)`. The haste is read through the layers rather than off the card
/// file, and the Elf across the table is the control — the offer names it,
/// because "target creature" is not "target creature you control", and the
/// seat whose creature was not named keeps its printed body and no keyword.
#[test]
fn flowstone_strike_gives_one_creature_plus_one_minus_one_and_haste() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), rootbreaker_wurm(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[flowstone_strike()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the Wurm is out");
    let bystander =
        on_battlefield(&engine, p0, llanowar_elves()).expect("a second creature of mine");
    let theirs =
        on_battlefield(&engine, p1, llanowar_elves()).expect("a creature across the table");
    assert_eq!(pt(&engine, wurm), (6, 6), "a printed 6/6 before the spell");
    assert!(
        !keywords(&engine, wurm).contains(KeywordSet::HASTE),
        "and no haste until the sentence grants it"
    );

    // The two Mountains and no creature: the Elves are named as the printing
    // kept back, so "exactly two" is two red rather than a green the pool
    // would have to account for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two tapped Mountains, and neither Elf paid in"
    );
    cast_with_floating(&mut engine, p0, flowstone_strike());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the casting seat aims it");
    assert!(
        options.contains(&wurm) && options.contains(&bystander),
        "both creatures you control are on the menu: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is not \"target creature you control\": {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, wurm),
        (7, 5),
        "+1/-1 on the creature that was named — a (7, 7) would be a toughness \
         the card never lowers, and a (5, 7) would have the two halves swapped"
    );
    assert!(
        keywords(&engine, wurm).contains(KeywordSet::HASTE),
        "and the same sentence grants haste until end of turn"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the creature the spell did not name keeps its printed body"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::HASTE),
        "and the granted keyword reaches the target and no other"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "nor across the table: one creature was named, and it was not this one"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "the pump is one creature, wherever it stood"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} came out of the pool"
    );
}
