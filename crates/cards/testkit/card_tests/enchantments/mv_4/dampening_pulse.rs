//! `cards/enchantments/mv_4/dampening_pulse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dampening_pulse() -> CardIndex {
    card_index("fe3c81b8-9bca-4760-b677-02bfa7e6137b")
}

/// "Creatures your opponents control get -1/-0": theirs lose one power and
/// no toughness, mine are untouched.
#[test]
fn dampening_pulse_shrinks_only_opposing_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dampening_pulse(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, theirs), (1, 2), "-1/-0 on an opposing creature");
    assert_eq!(pt(&engine, mine), (2, 2), "my own creature is untouched");
}
