//! `cards/creatures/mv_3/plague_rats.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Plague Rats — "Plague Rats's power and toughness are each equal to the
/// number of creatures named Plague Rats on the battlefield," counted
/// across both sides of the table.
#[test]
fn plague_ratss_power_and_toughness_count_every_plague_rats_on_the_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[plague_rats()])
        .battlefield(1, &[plague_rats()])
        .start();
    keep_mulligans(&mut engine);
    let mine = on_battlefield(&engine, p0, plague_rats()).expect("seated");
    let theirs = on_battlefield(&engine, p1, plague_rats()).expect("seated");
    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "two on the battlefield, whoever controls them"
    );
    assert_eq!(pt(&engine, theirs), (2, 2));
}
