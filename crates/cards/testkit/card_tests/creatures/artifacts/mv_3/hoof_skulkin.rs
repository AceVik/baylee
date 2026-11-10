//! `cards/creatures/artifacts/mv_3/hoof_skulkin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hoof_skulkin() -> CardIndex {
    card_index("7f60549f-7a0a-4009-a51a-d3f51214be11")
}

/// Hoof Skulkin — "{3}: Target green creature gets +1/+1 until end of turn."
/// The menu holds green creatures of either seat and not the colorless
/// Skulkin itself.
#[test]
fn hoof_skulkin_pumps_a_green_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                hoof_skulkin(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let skulkin = on_battlefield(&engine, p0, hoof_skulkin()).expect("seated");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");
    assert_eq!(pt(&engine, mine), (1, 1));
    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, hoof_skulkin(), 0);
    let options = aim_at(&mut engine, p0, mine);
    assert!(options.contains(&mine) && options.contains(&theirs));
    assert!(!options.contains(&skulkin), "the Skulkin is colorless");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, mine), (2, 2));
    assert_eq!(pt(&engine, theirs), (1, 1));
    assert_eq!(pt(&engine, skulkin), (2, 2));
}
