//! `cards/creatures/mv_4/blessed_orator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blessed Orator is a {3}{W} 1/4 printing one line: "Other creatures
/// **you control** get +0/+1." Two words carry the card and each needs a
/// bystander of its own: "other" is the Orator itself, which must stay the
/// 1/4 it prints rather than becoming a 1/5, and "you control" is the Elf
/// across the table, which must stay a printed 1/1 while the Elf on this
/// side becomes a 1/2. Four Plains pay the {3}{W} exactly — the Elves are
/// named as the printing kept back — so the board the three numbers are read
/// off is the board the cast actually left behind.
#[test]
fn blessed_orator_pumps_the_other_creatures_you_control_and_not_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .hand(0, &[blessed_orator()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the Orator");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and so is the one across the table"
    );

    // The Elves are named as the things kept back because a mana creature
    // tapped into the pool would make "four" a claim about five sources.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Plains, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, blessed_orator());
    pass_until(&mut engine, stack_is_empty);
    let orator = on_battlefield(&engine, p0, blessed_orator()).expect("the Orator resolved");

    assert_eq!(
        pt(&engine, orator),
        (1, 4),
        "\"other creatures\": the Orator does not pump itself, so it is the \
         body it prints and not a 1/5"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 2),
        "+0/+1 on the other creature you control"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for a creature you do not control"
    );
}
