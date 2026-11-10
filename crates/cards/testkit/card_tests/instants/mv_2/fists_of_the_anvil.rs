//! `cards/instants/mv_2/fists_of_the_anvil.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fists of the Anvil — {1}{R} Instant: "Target creature gets +4/+0 until
/// end of turn."
///
/// A pure pump is only itself when both halves of the printed sentence are
/// read off the board, so the creatures beside it are the controls: the Elf
/// across the table is a printed 1/1 that a filter which had lost "target"
/// would also have offered, and `(5, 1)` is the one body that reads both
/// printed numbers — a `(5, 5)` would mean a toughness the card never
/// prints, and `(1, 1)` that the pump never landed at all. The "until end of
/// turn" half is asserted a turn later, where the same Elf has to be back to
/// its printed body.
#[test]
fn fists_of_the_anvil_pumps_the_creature_it_targets_for_power_and_no_toughness() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[fists_of_the_anvil()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // Mana before the claim: `castable` is read off the pool and not off the
    // untapped Mountains (a pool survives until the step ends, CR 500.5, and
    // this whole cast happens inside one main phase).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "two Mountains and the Elf's own {{G}}, which is every source on this board"
    );
    cast_with_floating(&mut engine, p0, fists_of_the_anvil());

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
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" reaches either side of the table: {options:?}"
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
        (5, 1),
        "+4/+0 on the creature it targeted — a (5, 5) would be a toughness \
         the card does not print"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the Elf the spell did not name never moved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{1}}{{R}} came out of the pool the three sources filled"
    );
    assert!(
        in_graveyard(&engine, p0, fists_of_the_anvil()).is_some(),
        "an instant that resolved is in its owner's graveyard"
    );

    // "Until end of turn" is the half nothing about the offer or the pump
    // could show: a turn later the body is the printed one again.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "the pump lasted the turn it was cast and no longer"
    );
}
