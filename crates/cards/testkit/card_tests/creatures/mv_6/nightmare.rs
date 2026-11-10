//! `cards/creatures/mv_6/nightmare.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nightmare — Flying; "Nightmare's power and toughness are each equal to
/// the number of Swamps you control." Only its controller's Swamps count.
#[test]
fn nightmares_power_and_toughness_equal_the_swamps_its_controller_has() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[nightmare(), swamp(), swamp(), swamp()])
        .battlefield(1, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    let horse = on_battlefield(&engine, p0, nightmare()).expect("seated");
    assert!(keywords(&engine, horse).contains(KeywordSet::FLYING));
    assert_eq!(
        pt(&engine, horse),
        (3, 3),
        "three Swamps of its own; the opponent's Swamp does not count"
    );
}
