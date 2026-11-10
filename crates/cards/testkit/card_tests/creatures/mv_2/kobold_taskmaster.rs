//! `cards/creatures/mv_2/kobold_taskmaster.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kobold Taskmaster — {1}{R}, a 1/2 Kobold — prints exactly one sentence:
/// "Other Kobold creatures you control get +1/+0." Four words carry it and
/// each needs its own witness on the board: "Other" (a lone Taskmaster is a
/// Kobold its controller controls and has to stay 1/2), "Kobold" (the Elf
/// beside it is a creature you control and gains nothing), "you control" (a
/// third Taskmaster across the table has to stay 1/2) and the pump itself,
/// which only a second Kobold under the same seat can show — two Taskmasters
/// under one seat pump each other to 2/2 without either pumping itself.
#[test]
fn kobold_taskmaster_pumps_only_the_other_kobolds_its_controller_has() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[kobold_taskmaster()])
        .hand(0, &[kobold_taskmaster(), kobold_taskmaster()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let theirs = on_battlefield(&engine, p1, kobold_taskmaster()).expect("their Kobold is out");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "a printed 1/1 with nothing cast yet"
    );
    assert_eq!(pt(&engine, theirs), (1, 2), "and their printed 1/2");

    // Four Mountains pay for both Taskmasters — {1}{R} each — inside this one
    // main phase (CR 500.5), and the Elf is named as the source kept back so
    // that "four" is a count of four lands and not of a mana creature that
    // `tap_all_mana_but` would have drunk along with them.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains, and the Elf holds its {{G}}"
    );

    // The first copy alone: a Kobold its own controller controls, and the word
    // "Other" is what keeps the static off it.
    cast_with_floating(&mut engine, p0, kobold_taskmaster());
    pass_until(&mut engine, stack_is_empty);
    let lone = all_on_battlefield(&engine, p0, kobold_taskmaster());
    assert_eq!(lone.len(), 1, "one Taskmaster resolved");
    assert_eq!(
        pt(&engine, lone[0]),
        (1, 2),
        "\"Other\": the Taskmaster is a Kobold its controller controls, so it \
         does not pump itself"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "\"Kobold\": a creature you control of some other type gains nothing"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 2),
        "\"you control\": their Taskmaster is a Kobold and it is other, and it \
         still takes nothing"
    );

    // The second copy is the pump's only witness: no other card on this board
    // grants +1/+0 to anything.
    cast_with_floating(&mut engine, p0, kobold_taskmaster());
    pass_until(&mut engine, stack_is_empty);
    let pair = all_on_battlefield(&engine, p0, kobold_taskmaster());
    assert_eq!(pair.len(), 2, "the second copy resolved beside the first");
    assert_eq!(
        pt(&engine, pair[0]),
        (2, 2),
        "another Kobold you control: 1/2 plus the printed +1/+0"
    );
    assert_eq!(
        pt(&engine, pair[1]),
        (2, 2),
        "and the same read from the other side of the pair"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "two {{1}}{{R}} casts came out of the four Mountains"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the Elf is still a printed 1/1: the subtype filter is read and not \
         skipped"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 2),
        "and their Kobold gains nothing from a static it is not under"
    );
}
