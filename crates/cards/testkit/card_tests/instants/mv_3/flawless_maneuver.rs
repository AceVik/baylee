//! `cards/instants/mv_3/flawless_maneuver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flawless Maneuver: "Creatures you control gain indestructible until end
/// of turn." Hard-cast for {2}{W}: my creature has it, theirs does not.
#[test]
fn flawless_maneuver_makes_my_creatures_indestructible_and_not_theirs() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4703, plains())
        .battlefield(0, &[plains(), plains(), plains(), serra_angel()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[flawless_maneuver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, serra_angel()).expect("my Angel");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves");
    assert!(!keywords(&engine, mine).contains(KeywordSet::INDESTRUCTIBLE));
    cast_from_hand(&mut engine, p0, flawless_maneuver());
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, mine).contains(KeywordSet::INDESTRUCTIBLE));
    assert!(!keywords(&engine, theirs).contains(KeywordSet::INDESTRUCTIBLE));
}
