//! `cards/lands/check/castle_embereth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Castle Embereth: "Castle Embereth enters tapped unless you control a Mountain." / "{T}: Add {R}." / "{1}{R}{R}, {T}: Creatures you control get +1/+0 until end of turn."
/// Under `Coverage::Implemented`, controlling a Mountain allows Castle Embereth to enter untapped.
/// Activating its second ability for `{1}{R}{R}` and tapping gives creatures you control +1/+0 until end of turn.
#[test]
fn castle_embereth_enters_untapped_with_mountain_and_pumps_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(102, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), quiet_creature()])
        .hand(0, &[castle_embereth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let castle = play_land(&mut engine, p0, castle_embereth());
    assert!(!entered_tapped(&engine, castle));

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("creature deployed");
    assert_eq!(pt(&engine, elf), (1, 1));

    tap_mana_except(&mut engine, p0, castle);
    activate(&mut engine, p0, castle_embereth(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elf), (2, 1));
    assert!(is_tapped(&engine, castle));
}
