//! `cards/instants/mv_2/predator_s_strike.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Predator's Strike prints one sentence: "Target creature gets +3/+3 and gains
/// trample until end of turn." Both halves are read off the creature the spell
/// *named*, while a second creature stands across the table — "target creature"
/// is neither "target creature you control" nor "creatures you control", so the
/// offer holding both is what makes the 4/4 and the printed 1/1 beside it a
/// filter rather than a board-wide buff. The land on that same menu is the other
/// word struck: `Filter::CREATURE` is read and not skipped. Two Forests pay the
/// `{1}{G}` and are read empty afterwards, so the pump is something that was
/// bought rather than a board that happened to be big.
#[test]
fn predators_strike_pumps_and_tramples_the_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[predators_strike()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let land = on_battlefield(&engine, p0, forest()).expect("a Forest is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the spell");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "and nothing has granted it a keyword yet"
    );

    // The two Forests pay the {1}{G}, and the Elves are named as the printing
    // kept back: the creature this spell is about is the one that must not have
    // been tapped for its own mana, and two Forests are exactly the cost.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Forests tapped, and no creature of mine paid in"
    );
    cast_with_floating(&mut engine, p0, predators_strike());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it is the one that aims it");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert_eq!(
        options.len(),
        2,
        "the two creatures on the table and nothing else: {options:?}"
    );
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Forest is a permanent and no creature: `Filter::CREATURE` is read, \
         not skipped: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (4, 4),
        "+3/+3 on the creature the spell targeted"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::TRAMPLE),
        "and trample, which the layers have to project: the Elves print none"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the creature the spell did not name is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "the spell reaches the creature it targeted and never across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{G}} came out of the two Forests the pool was read from"
    );
    assert!(
        in_graveyard(&engine, p0, predators_strike()).is_some(),
        "an instant that finished resolving goes to its owner's graveyard"
    );
}
