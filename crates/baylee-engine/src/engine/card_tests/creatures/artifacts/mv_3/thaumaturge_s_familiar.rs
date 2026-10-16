//! `cards/creatures/artifacts/mv_3/thaumaturge_s_familiar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn thaumaturge_s_familiar() -> CardIndex {
    card_index("74a1a484-7c35-466a-b097-9aaf6fc2073b")
}

/// Thaumaturge's Familiar — 1/3 flier; "When this creature enters, scry 1".
#[test]
fn thaumaturge_s_familiar_scries_one_on_entering_and_flies() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(&[thaumaturge_s_familiar()], &[forest(), forest(), forest()]);
    cast_from_hand(&mut engine, p0, thaumaturge_s_familiar());

    let before = answer_scry(&mut engine, 1, true);

    assert_scried(&engine, &before, 1, true);
    let bird = on_battlefield(&engine, p0, thaumaturge_s_familiar()).expect("it resolved");
    assert_eq!(pt(&engine, bird), (1, 3));
    assert!(keywords(&engine, bird).contains(KeywordSet::FLYING));
}

/// Flying: seat 1's ground creature cannot block it.
#[test]
fn thaumaturge_s_familiar_cannot_be_blocked_by_a_ground_creature() {
    let (engine, [bird, ground], options) =
        evasion_probe(thaumaturge_s_familiar(), &[quiet_creature()]);
    let blocker = on_battlefield(&engine, PlayerId::new(1), quiet_creature()).expect("seated");
    assert!(may_block(&options, blocker, ground));
    assert!(!may_block(&options, blocker, bird));
}
