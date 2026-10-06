//! `cards/instants/mv_1/gaea_s_might.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gaea's Might prints one sentence: "{G} — Domain — Target creature gets
/// +1/+1 until end of turn for each basic land type among lands you control."
/// The count is of *types* and not of lands, and it reads only lands **you**
/// control, so the board is built to fail either misreading at once: two
/// Forests and an Island under p0 are three lands carrying two basic land
/// types, while a Swamp and a Mountain across the table would make it four if
/// "you control" were skipped. A 1/1 that ends as a 3/3 is the only answer
/// those two traps leave standing — a per-land count would read 4/4 and a
/// table-wide count 5/5.
#[test]
fn gaeas_might_pumps_by_basic_land_types_among_your_own_lands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), island(), quiet_creature()])
        .hand(0, &[gaea_s_might()])
        .battlefield(1, &[swamp(), mountain(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the spell");

    cast_from_hand(&mut engine, p0, gaea_s_might());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster aims it");
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
        .expect("the Elf was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (3, 3),
        "Forest and Island are two basic land types — not the three lands \
         that carry them, and not the four types the whole table has"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the creature the spell did not name is untouched"
    );
}
